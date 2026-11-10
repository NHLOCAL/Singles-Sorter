package com.nhlocal.singlesorter.app;
import java.util.HashSet;
/** A complete fresh parent listing, not a failed object query, can prove a released temporary child absent. */
final class RetainedNamespaceListing {
 private final String expectedId,expectedName;private final HashSet<String> ids=new HashSet<>();private boolean present;
 RetainedNamespaceListing(String id,String name){expectedId=id;expectedName=name;}
 void child(String id,String name){if(id==null||name==null||id.length()>4096||name.length()>1024||ids.size()>=10000||!ids.add(id))throw new IllegalArgumentException("RESERVED_LIST_INVALID");if(expectedId.equals(id)||expectedName.equalsIgnoreCase(name))present=true;}
 boolean absent(boolean loading,boolean error){return !loading&&!error&&!present;}
}
