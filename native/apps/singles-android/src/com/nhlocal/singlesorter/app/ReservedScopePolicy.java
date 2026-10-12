package com.nhlocal.singlesorter.app;
/** Exact absent temporary release only; unknown/missing originals keep their containing namespace. */
final class ReservedScopePolicy {
 private ReservedScopePolicy(){}
 // 0: exact temporary released; 1: reserve containing parent; 2: reserve observed owned directory.
 static int select(int kind,boolean checkedTemporaryRelease,boolean completeParentAbsence){if(kind!=1&&kind!=2&&checkedTemporaryRelease&&completeParentAbsence)return 0;return kind==2?2:1;}
 static void requireDirectory(int freshKind)throws java.io.IOException{if(freshKind!=2)throw new java.io.IOException("RESERVED_PARENT_UNAVAILABLE");}
}
