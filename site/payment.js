export const VND_PRESETS = [25000, 30000, 50000, 100000];
export const USD_PRESETS = [1, 2, 3, 5, 10];
export function parseRate(data, now=Date.now()) {
 const value=data?.rates?.VND, updatedAt=data?.time_last_update_unix*1000;
 if(data?.result!=='success'||data.base_code!=='USD'||!Number.isFinite(value)||value<1000||value>100000||!Number.isFinite(updatedAt)||updatedAt>now+300000||now-updatedAt>172800000) throw new Error('rate');
 return Object.freeze({value,updatedAt,source:'ExchangeRate-API'});
}
export const RECIPIENT = Object.freeze({bank:'VIB', bin:'970441', account:'862565367', name:'NGUYEN NGOC HAN'});
export function quote(raw, currency, rate) {
  if (!['VND','USD'].includes(currency)) throw new Error('currency');
  let text=String(raw).trim();
  if(currency==='VND' && /^\d{1,3}(?:[., ]\d{3})+$/.test(text)) text=text.replace(/[., ]/g,'');
  const valid=currency==='VND' ? /^\d+$/ : /^\d+(?:[.,]\d{1,2})?$/;
  if(!valid.test(text)) throw new Error('invalid');
  const value=Number(text.replace(',','.'));
  if(currency==='USD' && (!Number.isFinite(rate)||rate<1000||rate>100000||value<1)) throw new Error('rate');
  const vnd=currency==='VND' ? value : Math.round(value*rate);
  if(!Number.isFinite(value)||!Number.isSafeInteger(vnd)||vnd<25000||vnd>100000000) throw new Error('range');
  return Object.freeze({currency,value,vnd,rate:currency==='USD'?rate:null});
}
export function qrUrl(q) {
  const checked=quote(q.value,q.currency,q.rate);
  const url=new URL(`https://img.vietqr.io/image/${RECIPIENT.bin}-${RECIPIENT.account}-qr_only.png`);
  url.search=new URLSearchParams({amount:String(checked.vnd),addInfo:'AZR SUPPORT',accountName:RECIPIENT.name}).toString();
  return url.href;
}
