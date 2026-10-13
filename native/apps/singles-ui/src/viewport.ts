/** Use the visible, unzoomed viewport for keyboard resizing. Pinch zoom must not
 * resize/reflow the document underneath the user's magnification gesture. */
export function viewportHeight(inner:number,visual?:{height:number;scale:number}|null):number {
 const candidate=visual&&Math.abs(visual.scale-1)<.01?visual.height:inner;
 return Number.isFinite(candidate)&&candidate>0?candidate:Math.max(1,inner);
}
export function installViewport(app:HTMLElement){
 const update=()=>{
  app.style.setProperty('--viewport-height',`${viewportHeight(window.innerHeight,window.visualViewport)}px`);
  app.classList.toggle('reflow',parseFloat(getComputedStyle(document.documentElement).fontSize)>21);
 };
 window.addEventListener('resize',update);
 window.visualViewport?.addEventListener('resize',update);
 const observer=new ResizeObserver(update);observer.observe(document.documentElement);
 const header=app.querySelector('header');if(header)observer.observe(header);
 update();
 return()=>{window.removeEventListener('resize',update);window.visualViewport?.removeEventListener('resize',update);observer.disconnect();};
}
