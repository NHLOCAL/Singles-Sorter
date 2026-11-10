import type {ProfileHost} from './contracts';
/** Showing a dialog is not acknowledgement. Native window/process exit does not mark it seen. */
export class FirstRunGate {
 pending=false;
 private writing=false;
 private version:string|null=null;
 constructor(private host:Pick<ProfileHost,'getFirstRunStatus'|'markFirstRunSeen'>){}
 async load():Promise<boolean>{const status=await this.host.getFirstRunStatus();this.version=status.version;this.pending=!status.seen;return this.pending;}
 async acknowledge():Promise<void>{
  if(!this.pending||this.writing)return;
  this.writing=true;
  try{const status=await this.host.markFirstRunSeen();if(!status.seen||status.version!==this.version)throw new Error('First-run acknowledgement was not verified');this.pending=false;}
  finally{this.writing=false;}
 }
}
