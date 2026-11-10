// Run against an installed QA APK's real WebView, with adb-forwarded DevTools.
// Usage: node tests/check_webview_spacing.mjs [http://127.0.0.1:9223]
// No mocked host or replacement page is used.
import assert from 'node:assert/strict';

const endpoint=process.argv[2]??'http://127.0.0.1:9223';
const pages=await (await fetch(`${endpoint}/json/list`)).json();
const page=pages.find(page=>page.url==='https://appassets.androidplatform.net/assets/index.html');
assert.ok(page,'Open the packaged app in a QA build');
const socket=new WebSocket(page.webSocketDebuggerUrl);
await new Promise(resolve=>socket.addEventListener('open',resolve));
let next=0;
async function evaluate(expression){
 const id=++next;
 const response=await new Promise((resolve,reject)=>{
  const timer=setTimeout(()=>reject(new Error('WebView inspection timed out')),10000);
  const receive=event=>{const reply=JSON.parse(event.data);if(reply.id!==id)return;clearTimeout(timer);socket.removeEventListener('message',receive);resolve(reply);};
  socket.addEventListener('message',receive);
  socket.send(JSON.stringify({id,method:'Runtime.evaluate',params:{expression,returnByValue:true,awaitPromise:true}}));
 });
 assert.equal(response.error,undefined);
 assert.equal(response.result.exceptionDetails,undefined);
 return response.result.result.value;
}
const measure=`(()=>{
 const visible=e=>e.getBoundingClientRect().width>0&&e.getBoundingClientRect().height>0;
 const distance=(a,b)=>{a=a.getBoundingClientRect();b=b.getBoundingClientRect();return Math.max(a.left-b.right,b.left-a.right)};
 const options=[...document.querySelectorAll('.option')].filter(visible).map(label=>{
  const input=label.querySelector('input'),text=label.querySelector('span');
  return input&&text?{id:input.id,gap:distance(input,text)}:null;
 }).filter(Boolean);
 const folders=[...document.querySelectorAll('.folder-row')].filter(visible).map(row=>distance(row.querySelector('input'),row.querySelector('button')));
 const rows=[...document.querySelectorAll('.options .option-row')].filter(visible).map(row=>({height:row.getBoundingClientRect().height,margin:parseFloat(getComputedStyle(row).marginTop)}));
 const icons=[...document.querySelectorAll('button')].filter(visible).map(button=>{
  const icon=button.querySelector('.app-icon'),text=button.querySelector('span');
  return icon&&text&&visible(icon)&&visible(text)?{id:button.id,gap:distance(icon,text)}:null;
 }).filter(Boolean);
 return {options,folders,rows,icons,viewport:innerWidth,overflow:document.documentElement.scrollWidth-innerWidth};
})()`;
try{
 const setup=await evaluate(measure);
 assert.ok(setup.options.length>=5,'Start on the Sort setup screen');
 const verify=metrics=>{
  assert.ok(metrics.overflow<=1,JSON.stringify(metrics));
  for(const option of metrics.options)assert.ok(option.gap>=9.5,JSON.stringify(option));
  for(const gap of metrics.folders)assert.ok(gap>=7.5,`Folder control gap: ${gap}`);
  for(const icon of metrics.icons)assert.ok(icon.gap>=5.5,JSON.stringify(icon));
 };
 verify(setup);
 for(const row of setup.rows){assert.ok(row.height>=48);assert.equal(row.margin,0);}
 await evaluate(`document.getElementById('open-settings').click()`);
 const settings=await evaluate(measure);
 verify(settings);
 assert.ok(settings.options.some(option=>option.id==='duet-first'));
 await evaluate(`document.getElementById('advanced-dialog').close()`);
 console.log(JSON.stringify({setup,settings,passed:true},null,2));
}finally{socket.close();}
