import type {PersonalList,ProfileHost,ImportSelection,ImportPreview,ImportEncoding,ImportEdit} from './contracts';
import {errorText} from './state';
/** Ordered rows only. Never trim, deduplicate, sort or infer singer aliases. */
export class PersonalListController {
 snapshot:PersonalList|null=null;
 draft:string[][]=[];
 busy=false;
 error:string|null=null;
 notice:string|null=null;
 selection:ImportSelection|null=null;
 preview:ImportPreview|null=null;
 selectedEncoding:ImportEncoding='Utf8';
 importDraft:string[][]=[];
 private importDeleted=new Set<number>();
 private importCells=new Map<string,Extract<ImportEdit,{kind:'set'}>>();
 private installImport(preview:ImportPreview|null){this.preview=preview;this.importDraft=preview?.rows.map(row=>[...row])??[];this.importDeleted.clear();this.importCells.clear();}
 get importRows(){return this.importDraft.map((row,index)=>({row,index})).filter(({index})=>!this.importDeleted.has(index));}
 get importEdits():ImportEdit[]{return [...this.importCells.values()].filter(edit=>!this.importDeleted.has(edit.index)).sort((a,b)=>a.index-b.index||a.column-b.column).map(edit=>({...edit} as ImportEdit)).concat([...this.importDeleted].sort((a,b)=>a-b).map(index=>({kind:'delete' as const,index})));}
 editImport(index:number,column:0|1,value:string){if(this.busy||!this.preview||!this.importDraft[index]||this.importDeleted.has(index))return;const row=[...this.importDraft[index]];while(row.length<=column)row.push('');row[column]=value;this.importDraft[index]=row;const key=`${index}:${column}`;if(this.preview.rows[index][column]===value)this.importCells.delete(key);else this.importCells.set(key,{kind:'set',index,column,value});this.error=null;}
 deleteImport(index:number){if(this.busy||!this.preview||!this.importDraft[index])return;this.importDeleted.add(index);for(const [key,edit]of this.importCells)if(edit.index===index)this.importCells.delete(key);this.emit();}
 private listeners=new Set<()=>void>();
 constructor(private host:ProfileHost,private changed:()=>void){}
 subscribe(listener:()=>void){this.listeners.add(listener);return()=>this.listeners.delete(listener);}
 private emit(){for(const listener of this.listeners)listener();}
 private install(value:PersonalList){this.snapshot=value;this.draft=value.rows.map(row=>[...row]);}
 async open(){if(this.busy)return;this.busy=true;this.error=null;this.notice=null;this.emit();try{this.install(await this.host.getPersonalList());}catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}}
 cancel(){if(this.busy)return;if(this.snapshot)this.draft=this.snapshot.rows.map(row=>[...row]);this.selection=null;this.installImport(null);this.error=null;this.emit();}
 edit(index:number,column:0|1,value:string){if(this.busy||!this.draft[index])return;const row=[...this.draft[index]];while(row.length<2)row.push('');row[column]=value;this.draft[index]=row;this.error=null;}
 add(){if(this.busy)return;this.draft.push(['','']);this.emit();}
 remove(index:number){if(this.busy||!this.draft[index])return;this.draft.splice(index,1);this.emit();}
 async save(){
  if(this.busy||!this.snapshot)return;
  const invalid=this.draft.findIndex(row=>row.length!==2||row.some(field=>field.length===0));
  if(invalid!==-1){this.error=`שורה ${invalid+1} אינה מלאה: נדרשים שם אמן ושם תיקייה בלבד. תקן או מחק אותה במפורש; לא הוסרו שורות אוטומטית.`;this.emit();return;}
  this.busy=true;this.error=null;this.notice=null;this.emit();
  try{this.install(await this.host.savePersonalList(this.snapshot.revision,this.draft.map(row=>[...row])));this.notice='הרשימה נשמרה. סדר השורות והכפילויות נשמרו.';this.changed();}
  catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}
 }
 async selectImport(){
  if(this.busy)return;this.busy=true;this.error=null;this.notice=null;this.emit();
  try{const selected=await this.host.selectPersonalImport();if(selected){this.snapshot=await this.host.getPersonalList();this.selection=selected;this.installImport(selected.preview);this.selectedEncoding=selected.preview?.encoding??'Utf8';}}
  catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}
 }
 chooseEncoding(encoding:ImportEncoding){if(this.busy||this.selectedEncoding===encoding)return;this.selectedEncoding=encoding;this.installImport(null);this.error=null;this.emit();}
 async decode(encoding:ImportEncoding){
  if(this.busy||!this.selection)return;this.selectedEncoding=encoding;this.busy=true;this.error=null;this.installImport(null);this.emit();
  try{this.installImport(await this.host.previewPersonalImport(this.selection.token,encoding));}catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}
 }
 async confirmImport(){
  if(this.busy||!this.preview||!this.snapshot||!this.importRows.length||this.preview.encoding!==this.selectedEncoding)return;this.busy=true;this.error=null;this.emit();
  try{this.install(await this.host.confirmPersonalImport(this.preview.token,this.preview.encoding,this.snapshot.revision,this.importEdits));this.notice='השורות נוספו בסוף הרשימה, ללא מיון או הסרת כפילויות.';this.installImport(null);this.selection=null;this.changed();}
  catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}
 }
 async export(){if(this.busy)return;this.busy=true;this.error=null;this.notice=null;this.emit();try{const result=await this.host.exportPersonalList();if(result)this.notice=`הרשימה יוצאה: ${result.filename}`;}catch(e){this.error=errorText(e);}finally{this.busy=false;this.emit();}}
}
