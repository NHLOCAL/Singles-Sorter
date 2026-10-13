package com.nhlocal.singlesorter.app;
import java.io.IOException;
/** Producer-shaped intended children; observed names never prove historical attribution. */
final class RecoveryIntentLookup {
 private RecoveryIntentLookup(){}
 static String parent(String phase,String operation,String parent,String to,String name)throws IOException{
  if(!"intent".equals(phase)||name==null)return null;
  if(name.length()>1024)throw new IOException("REVIEW_NAME_LIMIT");
  String step=operation.substring(operation.indexOf('/')+1);
  boolean move=step.equals("move")||step.equals("move-stage")||step.equals("publish-move")||step.equals("backup-fix")||step.equals("publish-fix");
  String selected=move||step.matches("album-step-[0-9]{1,4}")&&to!=null?to:parent;
  if(selected==null||selected.length()>8192)throw new IOException("REVIEW_INTENT_PARENT");return selected;
 }
 static String status(int count){if(count<0)throw new IllegalArgumentException("negative count");return count==0?"name_not_observed":count==1?"named_candidate_observed":"ambiguous_name";}
}
