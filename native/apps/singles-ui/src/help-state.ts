/** Input-specific help lifetime. Mouse help never becomes pinned by a click. */
export type HelpOrigin='hover'|'keyboard'|'touch';
export class HelpState<Key> {
 active:Key|null=null;
 origin:HelpOrigin|null=null;
 private show(key:Key,origin:HelpOrigin){this.active=key;this.origin=origin;}
 close(){this.active=null;this.origin=null;}
 enter(key:Key,pointerType:string){
  if(pointerType==='touch')return;
  if(this.active===key&&this.origin==='keyboard')return;
  this.show(key,'hover');
 }
 leave(key:Key,pointerType:string){
  if(pointerType!=='touch'&&this.active===key&&this.origin==='hover')this.close();
 }
 focus(key:Key,pointerTriggered:boolean){if(!pointerTriggered)this.show(key,'keyboard');}
 blur(key:Key){if(this.active===key)this.close();}
 activate(key:Key,origin:HelpOrigin){
  if(origin==='touch'&&this.active===key&&this.origin==='touch')this.close();
  else this.show(key,origin);
 }
}
