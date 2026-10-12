package com.nhlocal.singlesorter.app;

import android.net.Uri;
import android.provider.DocumentsContract;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Producer-shaped Fix history. An interrupted boundary is readable but grants no acknowledgment. */
final class FixRecoveryEvidence {
    final int acknowledged,unresolved;
    private FixRecoveryEvidence(int acknowledged,int unresolved){this.acknowledged=acknowledged;this.unresolved=unresolved;}
    static FixRecoveryEvidence validate(JSONObject authority,JSONObject state,BooleanSupplier cancelled)throws Exception{
        if(FixProtocol.integer(authority,"fix_protocol",1)!=1||!FixProtocol.BACKUP_LEAF.equals(authority.getString("fix_backup_leaf")))throw invalid();
        JSONObject original=authority.getJSONObject("original_plan");NativeFixPlan plan=NativeFixPlan.parse(authority.getString("plan_id"),original,authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancelled);
        FixBindingIndex index=new FixBindingIndex(authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancelled);
        JSONArray events=state.getJSONArray("events");if(events.length()>10000)throw invalid();int acknowledged=0;Operation current=null;
        for(int i=0;i<events.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject event=events.getJSONObject(i);if(FixProtocol.integer(event,"sequence",9999)!=i)throw invalid();String operation=FixProtocol.string(event,"operation",100),phase=FixProtocol.string(event,"phase",40);if(!operation.matches("operation-[0-9]{1,5}(/[a-z-]+)?"))throw invalid();int slash=operation.indexOf('/'),number=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));String step=slash<0?null:operation.substring(slash+1);
            if(number!=acknowledged||number>=plan.operations.size())throw invalid();if(current==null)current=new Operation(plan.operations.get(number),index);current.event(phase,step,event.getJSONObject("detail"));
            if(current.verified){acknowledged++;current=null;}}
        JournalExportBounds.checkpoint(cancelled);return new FixRecoveryEvidence(acknowledged,plan.operations.size()-acknowledged);
    }
    private static IllegalArgumentException invalid(){return new IllegalArgumentException("FIX_RECOVERY_EVIDENCE");}
    static String uri(JSONObject entry)throws Exception{return DocumentsContract.buildDocumentUriUsingTree(Uri.parse(entry.getString("tree_uri")),entry.getString("document_id")).toString();}
    static void sameUri(String expected,Object actual)throws Exception{if(!(actual instanceof String)||!expected.equals(actual))throw invalid();}
    private static void boundReturn(Object value)throws Exception{if(value!=JSONObject.NULL&&(!(value instanceof String)||((String)value).length()>8192))throw invalid();}
    private static boolean same(Object left,Object right){return left==JSONObject.NULL?right==JSONObject.NULL:left.toString().equals(right.toString());}
    private static final class Operation {
        final NativeFixPlan.Fix fix;final JSONObject original;final String parent;String currentSource,preparedOutcome,stageDirectory,stageOutput,backupDirectory,backupMoved,backupFinal,stageName,preparedError;
        JSONObject staged,nativeEvidence,intent,returned,published;String activeStep;int phase,next;boolean prepared,renamed,backupObserved,backupRetained,destination,cleanup,verified,mismatch;
        Operation(NativeFixPlan.Fix fix,FixBindingIndex index)throws Exception{this.fix=fix;original=fix.binding();currentSource=uri(index.entry(fix.source()));parent=uri(index.parent(fix.source()));next=fix.rename?0:1;}
        void event(String event,String step,JSONObject detail)throws Exception{
            if(verified||mismatch)throw invalid();
            if(step==null){root(event,detail);return;}
            if(!prepared)throw invalid();String expected=expectedStep();if(!step.equals(expected))throw invalid();
            if(event.equals("intent")){if(phase!=0)throw invalid();checkIntent(step,detail);intent=detail;activeStep=step;phase=1;return;}
            if(!step.equals(activeStep))throw invalid();
            if(event.equals("returned")){if(phase!=1)throw invalid();if(step.equals("write-fix-stage")){FixProtocol.binding(detail,FixProtocol.STAGED_LIMIT);}else if(step.equals("remove-empty-fix-stage")){FixProtocol.bool(detail,"removed");}else{boundReturn(detail.get("uri"));}returned=detail;phase=2;return;}
            if(event.equals("effect-mismatch")){if(phase!=2||detail.length()==0)throw invalid();mismatch=true;return;}
            if(!event.equals("observed")||phase!=2)throw invalid();checkObserved(step,detail);phase=0;activeStep=null;intent=null;returned=null;next++;
        }
        private String expectedStep(){
            switch(next){case 0:return "rename-fix";case 1:return "create-fix-stage";case 2:return "create-fix-output";case 3:return "write-fix-stage";case 4:return "create-fix-backup";case 5:return "backup-fix";case 6:return "rename-fix-backup";case 7:return "publish-fix";case 8:return "remove-empty-fix-stage";default:return "no-more-effects";}
        }
        private void root(String event,JSONObject detail)throws Exception{
            if(phase!=0)throw invalid();
            if(event.equals("fix-prepared")){
                if(prepared)throw invalid();sameUri(currentSource,detail.get("source"));sameUri(parent,detail.get("parent"));if(!FixProtocol.sameBinding(original,detail.getJSONObject("original_binding")))throw invalid();preparedOutcome=detail.getString("outcome");
                if(!same(fix.failure(),detail.get("metadata_failure")))throw invalid();
                if(preparedOutcome.equals("metadata_failure")){if(fix.failure()==JSONObject.NULL||fix.save||detail.get("native_evidence")!=JSONObject.NULL||detail.get("native_error")!=JSONObject.NULL)throw invalid();}
                else if(preparedOutcome.equals("unchanged")){if(!fix.save||fix.repairs().length()!=0||detail.get("native_evidence")!=JSONObject.NULL||detail.get("native_error")!=JSONObject.NULL)throw invalid();}
                else if(preparedOutcome.equals("unsupported_tags")){String error=FixProtocol.string(detail,"native_error",8192);preparedError=error;if(!fix.save||fix.repairs().length()==0||!error.startsWith("Unsupported(")||!error.endsWith(")")||detail.get("native_evidence")!=JSONObject.NULL)throw invalid();}
                else if(preparedOutcome.equals("staged")){if(!fix.save||detail.get("native_error")!=JSONObject.NULL)throw invalid();nativeEvidence=detail.getJSONObject("native_evidence");staged=FixProtocol.nativeEvidence(nativeEvidence,original,fix.repairs());}
                else throw invalid();prepared=true;return;
            }
            if(!prepared)throw invalid();
            if(event.equals("fix-renamed")){if(!fix.rename||next!=1||renamed)throw invalid();sameUri(currentSource,detail.get("uri"));if(!FixProtocol.sameBinding(original,detail.getJSONObject("binding"))||!fix.intendedName.equals(detail.getString("name")))throw invalid();renamed=true;return;}
            if(fix.rename&&!renamed)throw invalid();
            if(event.equals("fix-backup-observed")){if(next!=6||backupObserved)throw invalid();backupEvidence(detail,backupMoved);backupObserved=true;return;}
            if(event.equals("fix-backup-retained")){if(next!=7||!backupObserved||backupRetained)throw invalid();backupEvidence(detail,backupFinal);if(!FixProtocol.BACKUP_LEAF.equals(detail.getString("name")))throw invalid();backupRetained=true;return;}
            if(event.equals("destination-observed")){if(next!=8||!backupRetained||destination)throw invalid();checkResult(detail,true);published=detail;destination=true;return;}
            if(event.equals("verified")){
                if(preparedOutcome.equals("staged")){if(next!=9||!cleanup||!destination||!backupRetained)throw invalid();checkResult(detail,true);for(String key:new String[]{"destination","name","bytes","sha256","backup","backup_bytes","renamed","repaired","unchanged","status"})if(!same(detail.get(key),published.get(key)))throw invalid();}
                else{if(next!=1)throw invalid();checkResult(detail,false);}verified=true;return;
            }
            throw invalid();
        }
        private void backupEvidence(JSONObject detail,String expected)throws Exception{sameUri(expected,detail.get("uri"));if(!FixProtocol.sameBinding(original,detail.getJSONObject("binding"))||!fix.originalName.equals(detail.getString("original_name"))||!fix.intendedName.equals(detail.getString("intended_name")))throw invalid();}
        private void checkIntent(String step,JSONObject detail)throws Exception{
            if(next>0&&fix.rename&&!renamed)throw invalid();if(next>0&&!preparedOutcome.equals("staged"))throw invalid();
            switch(step){
                case "rename-fix":sameUri(currentSource,detail.get("source"));sameUri(parent,detail.get("parent"));name(detail,fix.intendedName);break;
                case "create-fix-stage":create(detail,parent,".singles-fix-",true);stageName=detail.getString("name");break;
                case "create-fix-output":sameUri(stageDirectory,detail.get("parent"));name(detail,fix.intendedName);if(!"application/octet-stream".equals(detail.getString("mime")))throw invalid();break;
                case "write-fix-stage":sameUri(stageOutput,detail.get("stage"));if(!FixProtocol.sameBinding(staged,detail.getJSONObject("binding")))throw invalid();break;
                case "create-fix-backup":create(detail,parent,".singles-original-",true);break;
                case "backup-fix":move(detail,currentSource,parent,backupDirectory,fix.intendedName);break;
                case "rename-fix-backup":if(!backupObserved)throw invalid();sameUri(backupMoved,detail.get("source"));sameUri(backupDirectory,detail.get("parent"));name(detail,FixProtocol.BACKUP_LEAF);break;
                case "publish-fix":if(!backupRetained)throw invalid();move(detail,stageOutput,stageDirectory,parent,fix.intendedName);break;
                case "remove-empty-fix-stage":if(!destination)throw invalid();sameUri(stageDirectory,detail.get("owned_empty_directory"));sameUri(parent,detail.get("parent"));name(detail,stageName);break;
                default:throw invalid();
            }
        }
        private void checkObserved(String step,JSONObject detail)throws Exception{
            if(step.equals("write-fix-stage")){sameUri(stageOutput,detail.get("uri"));if(!FixProtocol.sameBinding(staged,detail.getJSONObject("binding"))||!FixProtocol.sameBinding(staged,returned))throw invalid();return;}
            if(step.equals("remove-empty-fix-stage")){if(!FixProtocol.bool(returned,"removed")||!FixProtocol.bool(detail,"owned_empty_stage_absent"))throw invalid();cleanup=true;return;}
            String observed=FixProtocol.string(detail,"uri",8192);Object raw=returned.get("uri");if(!(raw instanceof String))throw invalid();Uri returnedUri=Uri.parse((String)raw),observedUri=Uri.parse(observed);
            if(!ProviderReferences.AUTHORITY.equals(returnedUri.getAuthority())||!"content".equals(returnedUri.getScheme())||!DocumentsContract.getDocumentId(returnedUri).equals(DocumentsContract.getDocumentId(observedUri)))throw invalid();
            JSONObject metadata=detail.getJSONObject("metadata");String expectedName=intent.getString("name");if(!expectedName.equals(metadata.getString("name"))||!DocumentsContract.getDocumentId(observedUri).equals(metadata.getString("id")))throw invalid();
            String tree=Uri.parse(parent).toString();Uri scoped=DocumentsContract.buildDocumentUriUsingTree(Uri.parse(tree),DocumentsContract.getDocumentId(observedUri));sameUri(scoped.toString(),observed);
            boolean directory=step.equals("create-fix-stage")||step.equals("create-fix-backup");if(directory!=DocumentsContract.Document.MIME_TYPE_DIR.equals(metadata.getString("mime")))throw invalid();
            // Exact returned/scoped references are evidence. Fresh ancestry is a live-producer prerequisite.
            switch(step){case "rename-fix":currentSource=observed;break;case "create-fix-stage":stageDirectory=observed;break;case "create-fix-output":stageOutput=observed;break;case "create-fix-backup":backupDirectory=observed;break;case "backup-fix":backupMoved=observed;break;case "rename-fix-backup":backupFinal=observed;break;case "publish-fix":currentSource=observed;break;default:throw invalid();}
        }
        private void checkResult(JSONObject detail,boolean replaced)throws Exception{
            String expectedStatus=preparedOutcome.equals("metadata_failure")?"metadata_failure":preparedOutcome.equals("unsupported_tags")?"unsupported_tags":"complete";
            sameUri(currentSource,detail.get("destination"));if(!fix.intendedName.equals(detail.getString("name"))||!expectedStatus.equals(detail.getString("status"))||FixProtocol.bool(detail,"renamed")!=renamed||FixProtocol.bool(detail,"repaired")!=replaced||FixProtocol.bool(detail,"unchanged")!=preparedOutcome.equals("unchanged")||FixProtocol.bool(detail,"copy"))throw invalid();
            JSONObject binding=new JSONObject().put("bytes",detail.get("bytes")).put("sha256",detail.get("sha256"));if(!FixProtocol.sameBinding(replaced?staged:original,binding))throw invalid();
            if(replaced){sameUri(backupFinal,detail.get("backup"));if(FixProtocol.integer(detail,"backup_bytes",FixProtocol.ORIGINAL_LIMIT)!=original.getLong("bytes"))throw invalid();}else if(detail.get("backup")!=JSONObject.NULL||FixProtocol.integer(detail,"backup_bytes",FixProtocol.ORIGINAL_LIMIT)!=0)throw invalid();
            if(!same(fix.failure(),detail.get("metadata_failure")))throw invalid();if(preparedOutcome.equals("unsupported_tags")){if(!preparedError.equals(detail.getString("native_error")))throw invalid();}else if(detail.get("native_error")!=JSONObject.NULL)throw invalid();
        }
        private void create(JSONObject detail,String parent,String prefix,boolean directory)throws Exception{sameUri(parent,detail.get("parent"));String name=detail.getString("name");if(!name.startsWith(prefix)||!JournalExportBounds.id(name.substring(prefix.length()))||directory&&!DocumentsContract.Document.MIME_TYPE_DIR.equals(detail.getString("mime")))throw invalid();}
        private void name(JSONObject detail,String expected)throws Exception{if(!expected.equals(detail.getString("name")))throw invalid();ProviderNames.require(expected);}
        private void move(JSONObject detail,String source,String from,String to,String name)throws Exception{sameUri(source,detail.get("source"));sameUri(from,detail.get("from"));sameUri(to,detail.get("to"));name(detail,name);}
    }
}
