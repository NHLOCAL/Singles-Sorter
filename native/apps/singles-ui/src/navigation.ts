import type {Mode} from './contracts';
export type Screen='setup'|'processing'|'results';
/** Presentation navigation only: returning never starts or repeats host work. */
export class WorkspaceNavigation {
 screen:Screen='setup';mode:Mode='Sort';
 start(mode:Mode){if(this.screen==='processing')return false;this.mode=mode;this.screen='processing';return true;}
 finish(hasResult:boolean){this.screen=hasResult?'results':'setup';}
 setup(){if(this.screen!=='processing')this.screen='setup';}
 results(hasResult:boolean){if(hasResult&&this.screen!=='processing')this.screen='results';}
}
