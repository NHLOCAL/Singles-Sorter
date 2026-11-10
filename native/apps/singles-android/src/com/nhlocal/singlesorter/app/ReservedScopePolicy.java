package com.nhlocal.singlesorter.app;
/** Exact absent temporary release only; unknown/missing originals keep their containing namespace. */
final class ReservedScopePolicy {
 private ReservedScopePolicy(){}
 // 0: exact temporary released; 1: reserve containing parent; 2: reserve observed owned directory.
 static int select(int kind,boolean checkedTemporaryRelease,boolean completeParentAbsence){if(kind!=1&&kind!=2&&checkedTemporaryRelease&&completeParentAbsence)return 0;return kind==2?2:1;}
 /** A provider-confirmed missing namespace protects nothing; only an unobservable or non-directory one refuses. */
 static boolean stillReserved(int freshKind)throws java.io.IOException{if(freshKind==0)return false;if(freshKind!=2)throw new java.io.IOException("RESERVED_PARENT_UNAVAILABLE");return true;}
 /** Negative filter for the one admitted provider, whose document IDs are hierarchical paths. A match is still confirmed by the provider. */
 static boolean mayContain(String parentId,String childId){int length=parentId.length();return childId.length()>length&&childId.regionMatches(true,0,parentId,0,length)&&(parentId.endsWith(":")||parentId.endsWith("/")||childId.charAt(length)=='/');}
}
