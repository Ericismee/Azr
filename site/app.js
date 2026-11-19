import {quote,qrUrl,VND_PRESETS,USD_PRESETS,parseRate,RECIPIENT} from './payment.js';
const $=id=>document.getElementById(id);
const initialVi=navigator.language?.toLowerCase().startsWith('vi');
let lang=initialVi?'vi':'en',currency=initialVi?'VND':'USD',selected=initialVi?25000:1,generation=0,current=null,fx=null,fxLoading=true;
const copy={
vi:{support:'Ủng hộ dự án',heading:'Tiếp sức cho Azr.',lead:'Một ly cà phê nhỏ, thêm động lực để Azr tốt hơn mỗi ngày.',voluntary:'Hoàn toàn tự nguyện. Không đăng ký định kỳ.',repo:'Tìm hiểu về Azr trên GitHub',oneTime:'Ủng hộ một lần qua VietQR',currency:'Đơn vị hiển thị',vndOption:'Việt Nam · VND',usdOption:'Quốc tế · USD tham chiếu',choose:'Chọn mức ủng hộ',custom:'Hoặc nhập số tiền khác',transfer:'Số tiền chuyển khoản',generate:'Tạo mã VietQR',bankNote:'VietQR nhận VND qua ứng dụng ngân hàng hỗ trợ. Không phải cổng thanh toán thẻ hoặc nhận USD quốc tế.',scan:'Quét mã để chuyển khoản',recipient:'Người nhận',bank:'Ngân hàng',account:'Số tài khoản',openQr:'Mở ảnh QR',retry:'Thử tải lại',verify:'Kiểm tra người nhận trong ứng dụng ngân hàng.',footer:'Cảm ơn bạn đã dành sự quan tâm cho dự án.',back:'Về GitHub',minimum:'Tối thiểu 25.000đ.',usdMinimum:'Từ 1 USD; số tiền quy đổi tối thiểu 25.000đ.',usdNote:'Quy đổi theo tỷ giá thị trường tham chiếu, cập nhật hằng ngày. VietQR nhận VND; đây không phải tỷ giá mua/bán của VIB hoặc cổng nhận USD quốc tế.',invalid:'Nhập số tiền hợp lệ, từ 25.000đ đến 100.000.000đ.',invalidUsd:'Nhập từ 1 đến 4.000 USD tham chiếu, tối đa 2 chữ số thập phân.',loading:'Đang tải QR…',ready:'Sẵn sàng',loadError:'Không tải được QR. Bạn có thể thử lại hoặc chuyển khoản bằng thông tin bên dưới.',copied:'Đã sao chép',copyFailed:'Hãy sao chép số tài khoản hiển thị.',placeholder:'Ví dụ: 40000'},
en:{support:'Support the project',heading:'Support Azr.',lead:'A small coffee. A little more momentum for Azr.',voluntary:'Always voluntary. No recurring subscription.',repo:'Explore Azr on GitHub',oneTime:'One-time support via VietQR',currency:'Display currency',vndOption:'Vietnam · VND',usdOption:'International · USD reference',choose:'Choose an amount',custom:'Or enter a custom amount',transfer:'Bank transfer amount',generate:'Generate VietQR',bankNote:'VietQR accepts VND through supported banking apps. This is not an international USD or card checkout.',scan:'Scan to make a bank transfer',recipient:'Recipient',bank:'Bank',account:'Account number',openQr:'Open QR image',retry:'Retry loading',verify:'Check the recipient in your banking app.',footer:'Thank you for supporting the project.',back:'Back to GitHub',minimum:'Minimum 25,000 VND.',usdMinimum:'From USD 1; converted minimum VND 25,000.',usdNote:'Conversion uses a daily reference market rate. VietQR settles in VND; this is not VIB’s buy/sell rate or an international USD checkout.',invalid:'Enter a valid whole amount from VND 25,000 to VND 100,000,000.',invalidUsd:'Enter USD 1 to USD 4,000 as a reference, with at most 2 decimal places.',loading:'Loading QR…',ready:'Ready',loadError:'The QR could not load. Retry or use the bank details below for a transfer.',copied:'Copied',copyFailed:'Please copy the displayed account number.',placeholder:'For example: 40000'}
};
Object.assign(copy.vi,{thanks:'Cảm ơn bạn!',scanHint:'Quét mã để ủng hộ Azr.',rateLoading:'Đang cập nhật tỷ giá USD/VND…',rateError:'Chưa lấy được tỷ giá. Hãy thử lại hoặc chọn VND.',rateTime:'Cập nhật',close:'Đóng',invalidUsd:'Nhập từ 1 USD, tối đa 2 chữ số thập phân; số VND quy đổi không vượt 100 triệu.'});
Object.assign(copy.en,{thanks:'Thank you!',scanHint:'Scan to support Azr.',rateLoading:'Updating the USD/VND rate…',rateError:'Rate unavailable. Retry or choose VND.',rateTime:'Updated',close:'Close',invalidUsd:'Enter at least USD 1, with up to 2 decimal places; the VND total must not exceed 100 million.'});
const t=key=>copy[lang][key];
const money=v=>new Intl.NumberFormat(lang==='vi'?'vi-VN':'en-US',{style:'currency',currency:'VND',maximumFractionDigits:0}).format(v);
function clearQr(){generation++;current=null;if($('qr-panel').open)$('qr-panel').close();document.body.classList.remove('modal-open');$('qr-error').textContent='';$('qr-image').removeAttribute('src');$('open-qr').removeAttribute('href');$('retry-qr').hidden=true;}
function amountQuote(){if(currency==='USD' && (!fx||Date.now()-fx.updatedAt>172800000))throw new Error('rate');return quote(selected??$('custom-amount').value,currency,fx?.value);}
function update(showError=false){
 $('amount-error').textContent='';
 try{const q=amountQuote();$('transfer-amount').textContent=money(q.vnd);$('generate').disabled=false;return q;}
 catch{$('transfer-amount').textContent='—';$('generate').disabled=true;if(currency==='USD'&&!fx)$('amount-error').textContent=t(fxLoading?'rateLoading':'rateError');else if(showError)$('amount-error').textContent=t(currency==='VND'?'invalid':'invalidUsd');return null;}
}
function presets(){
 $('presets').replaceChildren();
 for(const value of currency==='VND'?VND_PRESETS:USD_PRESETS){
  const b=document.createElement('button');b.type='button';b.className='preset';b.dataset.value=String(value);b.setAttribute('aria-pressed',String(selected===value));
  b.textContent=currency==='VND'?new Intl.NumberFormat('vi-VN').format(value):`$${value}`;
  b.addEventListener('click',()=>{clearQr();selected=value;$('custom-amount').value='';presets();update();});$('presets').append(b);
 }
}
function translate(){
 document.documentElement.lang=lang;document.title=lang==='vi'?'Ủng hộ Azr':'Support Azr';
 document.querySelectorAll('[data-i18n]').forEach(el=>el.textContent=t(el.dataset.i18n));
 $('language').textContent=lang==='vi'?'English':'Tiếng Việt';$('language').setAttribute('aria-label',lang==='vi'?'Switch to English':'Chuyển sang tiếng Việt');
 $('input-unit').textContent=currency;$('custom-amount').inputMode=currency==='VND'?'numeric':'decimal';$('custom-amount').placeholder=currency==='VND'?t('placeholder'):'1.50';
 $('minimum').textContent=t(currency==='VND'?'minimum':'usdMinimum');$('usd-note').hidden=currency!=='USD';$('usd-note').textContent=t('usdNote');
 $('close-qr').setAttribute('aria-label',t('close'));rateStatus();presets();update($('custom-amount').value.length>0);
 if(current){$('qr-amount').textContent=money(current.vnd);$('qr-status').textContent=$('qr-image').hidden?t('loading'):t('ready');}
}
function showQr(q){
 clearQr();current=q;const token=generation;const url=qrUrl(q);$('qr-panel').showModal();document.body.classList.add('modal-open');$('qr-status').textContent=t('loading');$('qr-amount').textContent=money(q.vnd);$('qr-image').hidden=true;$('open-qr').href=url;
 const image=new Image();image.referrerPolicy='no-referrer';
 image.onload=()=>{if(generation!==token)return;$('qr-image').src=url;$('qr-image').hidden=false;$('qr-status').textContent=$('qr-image').hidden?t('loading'):t('ready');};
 image.onerror=()=>{if(generation!==token)return;$('qr-status').textContent='';$('qr-error').textContent=t('loadError');$('retry-qr').hidden=false;};image.src=url;
}
$('currency').value=currency;
$('currency').addEventListener('change',()=>{clearQr();currency=$('currency').value;selected=currency==='VND'?25000:1;$('custom-amount').value='';translate();});
$('custom-amount').addEventListener('input',()=>{clearQr();selected=null;presets();update(true);});
$('language').addEventListener('click',()=>{lang=lang==='vi'?'en':'vi';translate();});
$('donate-form').addEventListener('submit',event=>{event.preventDefault();const q=update(true);if(q)showQr(q);});
$('retry-qr').addEventListener('click',()=>{if(current)showQr(current);});
$('close-qr').addEventListener('click',clearQr);
$('qr-panel').addEventListener('cancel',e=>{e.preventDefault();clearQr();});
$('qr-panel').addEventListener('click',e=>{const r=$('qr-panel').getBoundingClientRect();if(e.target===$('qr-panel')&&(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom))clearQr();});
function rateStatus(){
 $('fx-meta').hidden=currency!=='USD';$('retry-rate').hidden=fxLoading||!!fx;
 $('fx-status').textContent=fx?`1 USD = ${new Intl.NumberFormat(lang==='vi'?'vi-VN':'en-US',{maximumFractionDigits:2}).format(fx.value)} VND. ${t('rateTime')}: ${new Date(fx.updatedAt).toLocaleString(lang==='vi'?'vi-VN':'en-US')}. `:t(fxLoading?'rateLoading':'rateError')+' ';
}
async function loadRate(){
 fxLoading=true;rateStatus();update();
 try{const response=await fetch('https://open.er-api.com/v6/latest/USD',{signal:AbortSignal.timeout(10000),credentials:'omit',cache:'no-store'});if(!response.ok)throw Error('rate');const next=parseRate(await response.json());if(fx&&fx.value!==next.value)clearQr();fx=next;}
 catch{fx=null;if(currency==='USD')clearQr();}
 finally{fxLoading=false;rateStatus();update();}
}
$('retry-rate').addEventListener('click',loadRate);
translate();loadRate();setInterval(loadRate,3600000);

$('qr-panel').addEventListener('keydown',event=>{
 if(event.key!=='Tab')return;
 const items=[...$('qr-panel').querySelectorAll('button:not([disabled]),a[href],input:not([disabled]),[tabindex="0"]')].filter(el=>el.getClientRects().length);
 const first=items[0],last=items.at(-1);
 if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}
 else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
});
