package com.nhlocal.singlesorter.app;
/** Byte comparison, never proof of which actor performed a historical operation. */
final class RecoveryOriginalComparison {
 private RecoveryOriginalComparison(){}
 static boolean matches(long expectedBytes,String expectedHash,long currentBytes,String currentHash){
  return expectedBytes>=0&&expectedBytes==currentBytes&&expectedHash!=null&&expectedHash.matches("[0-9a-f]{64}")&&expectedHash.equals(currentHash);
 }
}
