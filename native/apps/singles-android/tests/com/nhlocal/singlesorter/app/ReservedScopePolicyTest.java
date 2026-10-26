package com.nhlocal.singlesorter.app;
public final class ReservedScopePolicyTest {
 private ReservedScopePolicyTest(){}
 public static void main(String[] args)throws Exception{int count=0;int[] kind={-1,0,1,2};for(boolean released:new boolean[]{false,true})for(boolean absent:new boolean[]{false,true})for(int value:kind){int expected=value!=1&&value!=2&&released&&absent?0:value==2?2:1;if(ReservedScopePolicy.select(value,released,absent)!=expected)throw new AssertionError();count++;}for(int value:kind){try{ReservedScopePolicy.requireDirectory(value);if(value!=2)throw new AssertionError();}catch(java.io.IOException expected){if(value==2||!"RESERVED_PARENT_UNAVAILABLE".equals(expected.getMessage()))throw new AssertionError();}count++;}System.out.println(count+" retained backup, lost-return and repeated-job namespace checks passed");}
}
