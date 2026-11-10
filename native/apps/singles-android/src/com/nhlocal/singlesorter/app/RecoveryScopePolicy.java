package com.nhlocal.singlesorter.app;
/** Unknown ancestry never establishes disjoint authority. */
final class RecoveryScopePolicy {
 private RecoveryScopePolicy(){}
 static boolean disjoint(boolean sameId,Boolean newContainsOld,Boolean oldContainsNew){return !sameId&&Boolean.FALSE.equals(newContainsOld)&&Boolean.FALSE.equals(oldContainsNew);}
}
