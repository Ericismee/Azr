import test from 'node:test';
import assert from 'node:assert/strict';
import {quote, qrUrl, VND_PRESETS, USD_PRESETS, parseRate} from '../payment.js';
test('latest preset amounts and minimum VND',()=>{
 assert.deepEqual(VND_PRESETS,[25000,30000,50000,100000]);
 assert.deepEqual(USD_PRESETS,[1,2,3,5,10]);
 assert.equal(quote('25000','VND').vnd,25000);
 assert.equal(quote('25.000','VND').vnd,25000);
 assert.equal(quote('1','USD',25703.83069).vnd,25704);
 assert.equal(quote('1.25','USD',25703.83069).vnd,32130);
});
test('reject empty, malformed, too small, fractional VND and excessive amounts',()=>{
 for(const v of ['', '0','24999','-50000','NaN','Infinity','1e5','25000.5','100000001']) assert.throws(()=>quote(v,'VND'));
 for(const v of ['','0.99','-1','1.001','4001']) assert.throws(()=>quote(v,'USD',25703.83069));
 assert.throws(()=>quote('25','EUR'));
});
test('QR always settles VND into the exact user-supplied VIB account',()=>{
 const u=new URL(qrUrl(quote('3','USD',25703.83069)));
 assert.equal(u.origin,'https://img.vietqr.io');
 assert.equal(u.pathname,'/image/970441-862565367-qr_only.png');
 assert.equal(u.searchParams.get('amount'),'77111');
 assert.equal(u.searchParams.get('accountName'),'NGUYEN NGOC HAN');
});

test('USD requires a valid external rate and preserves its quote snapshot',()=>{
 assert.throws(()=>quote('1','USD'));
 assert.throws(()=>quote('1','USD',NaN));
 assert.throws(()=>quote('1','USD',-1));
 const q=quote('2','USD',25703.83069);
 assert.equal(new URL(qrUrl(q)).searchParams.get('amount'),'51408');
});
test('external rate must be USD/VND, recent and finite',()=>{
 const now=Date.now(); const p={result:'success',base_code:'USD',rates:{VND:25703.83069},time_last_update_unix:Math.floor(now/1000)-60,time_next_update_unix:Math.floor(now/1000)+3600};
 assert.equal(parseRate(p,now).value,25703.83069);
 assert.throws(()=>parseRate({...p,base_code:'EUR'},now));
 assert.throws(()=>parseRate({...p,rates:{VND:NaN}},now));
 assert.throws(()=>parseRate({...p,time_last_update_unix:Math.floor(now/1000)-172801},now));
});
