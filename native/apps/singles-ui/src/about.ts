import {version} from '../package.json';
export const APP_VERSION=version;
export type ProjectLinkTarget='Source'|'License';
/** Display-only fallbacks. The native host independently owns its URL allowlist. */
export const PROJECT_LINKS:Record<ProjectLinkTarget,string>={
 Source:'https://github.com/NHLOCAL/Singles-Sorter',
 License:'https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE',
};
export function installAbout(root:HTMLElement,openLink:(target:ProjectLinkTarget)=>Promise<void>,openNotices?:()=>Promise<void>){
 root.querySelector<HTMLElement>('#about-version')!.textContent=APP_VERSION;
 const error=root.querySelector<HTMLElement>('#about-link-error')!;
 for(const button of root.querySelectorAll<HTMLButtonElement>('[data-project-link]'))button.addEventListener('click',async()=>{
  const target=button.dataset.projectLink as ProjectLinkTarget;
  if(target!=='Source'&&target!=='License')return;
  button.disabled=true;error.hidden=true;
  try{await openLink(target);}catch{error.textContent=`לא ניתן לפתוח כעת את הדפדפן. אפשר להעתיק את הכתובת: ${PROJECT_LINKS[target]}`;error.hidden=false;}
  finally{button.disabled=false;}
 });
 const notices=root.querySelector<HTMLButtonElement>('[data-local-notices]');
 if(notices&&openNotices){notices.hidden=false;notices.addEventListener('click',async()=>{notices.disabled=true;error.hidden=true;try{await openNotices();}catch{error.textContent='לא ניתן לפתוח כעת את הודעות הרישוי.';error.hidden=false;}finally{notices.disabled=false;}});}
 const pages=root.querySelectorAll<HTMLElement>('[data-info-page]');
 const buttons=root.querySelectorAll<HTMLButtonElement>('[data-help]');
 for(const button of buttons)button.addEventListener('click',()=>{
  for(const other of buttons)other.setAttribute('aria-pressed',String(other===button));
  for(const page of pages)page.hidden=page.dataset.infoPage!==button.dataset.help;
 });
}
