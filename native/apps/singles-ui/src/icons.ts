import source from './assets/fluent/folder_open_20_regular.svg?raw';
import folder from './assets/fluent/folder_20_regular.svg?raw';
import sort from './assets/fluent/arrow_sort_down_lines_20_regular.svg?raw';
import fix from './assets/fluent/wrench_20_regular.svg?raw';
import settings from './assets/fluent/settings_20_regular.svg?raw';
import save from './assets/fluent/save_20_regular.svg?raw';
import add from './assets/fluent/add_20_regular.svg?raw';
import help from './assets/fluent/question_circle_20_regular.svg?raw';
import more from './assets/fluent/more_horizontal_20_regular.svg?raw';
import close from './assets/fluent/dismiss_20_regular.svg?raw';
import info from './assets/fluent/info_20_regular.svg?raw';
import back from './assets/fluent/arrow_right_20_regular.svg?raw';
import next from './assets/fluent/arrow_left_20_regular.svg?raw';
import importIcon from './assets/fluent/arrow_import_20_regular.svg?raw';
import exportIcon from './assets/fluent/arrow_export_20_regular.svg?raw';
import deleteIcon from './assets/fluent/delete_20_regular.svg?raw';
const assets={source,folder,sort,fix,settings,save,add,delete:deleteIcon,import:importIcon,export:exportIcon,help,more,close,info,back,next};
export type IconName=keyof typeof assets;
/** Only trusted, locally vendored official assets enter this SVG parser. */
export function iconElement(name:IconName):SVGSVGElement{
 const parsed=new DOMParser().parseFromString(assets[name],'image/svg+xml').documentElement;
 const svg=document.importNode(parsed,true) as unknown as SVGSVGElement;
 svg.classList.add('app-icon');if(['source','import','export'].includes(name))svg.classList.add('rtl-mirror');
 svg.setAttribute('aria-hidden','true');svg.setAttribute('focusable','false');return svg;
}
export function setButtonLabel(button:HTMLElement,label:string,name:IconName){const span=document.createElement('span');span.textContent=label;button.replaceChildren(iconElement(name),span);}
export function installIcons(root:HTMLElement){
 const ids:Record<string,IconName>={menu:'more',sort:'sort',fix:'fix','source-picker':'source','target-picker':'folder',advanced:'settings','save-settings':'save','edit-list':'settings','import-list':'import','export-list':'export','list-add':'add','list-save':'save','confirm-import':'import','decode-import':'info','back-setup':'back','reopen-results':'next',previous:'back',next:'next','list-prev':'back','list-next':'next','import-prev':'back','import-next':'next'};
 for(const [id,name]of Object.entries(ids)){const button=root.querySelector<HTMLElement>(`#${id}`);if(button)setButtonLabel(button,id==='menu'?'':button.textContent??'',name);}
 for(const button of root.querySelectorAll<HTMLElement>('.help-badge'))button.replaceChildren(iconElement('help'));
 for(const button of root.querySelectorAll<HTMLElement>('.icon-button[aria-label^="סגור"],.icon-button[aria-label^="בטל"]'))button.replaceChildren(iconElement('close'));
}
