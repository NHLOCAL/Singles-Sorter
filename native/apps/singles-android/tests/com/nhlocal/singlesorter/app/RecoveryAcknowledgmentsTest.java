package com.nhlocal.singlesorter.app;
public final class RecoveryAcknowledgmentsTest {
 private RecoveryAcknowledgmentsTest(){}
 private static void rejects(Runnable test){try{test.run();throw new AssertionError();}catch(IllegalArgumentException expected){}}
 public static void main(String[] args){
  RecoveryAcknowledgments value=new RecoveryAcknowledgments(2);
  value.event(0,"intent","operation-0/publish-copy");value.event(1,"returned","operation-0/publish-copy");value.event(2,"observed","operation-0/publish-copy");value.event(3,"verified","operation-0");
  value.event(4,"intent","operation-1/write-stage");value.event(5,"returned","operation-1/write-stage");
  if(value.acknowledged()!=1||value.unresolved()!=1)throw new AssertionError();
  rejects(()->value.event(6,"verified","operation-0"));
  rejects(()->new RecoveryAcknowledgments(10001));
  rejects(()->new RecoveryAcknowledgments(1).event(1,"intent","operation-0"));
  rejects(()->new RecoveryAcknowledgments(1).event(0,"intent","operation-1"));
  rejects(()->new RecoveryAcknowledgments(1).event(0,"verified","operation-0"));
  rejects(()->new RecoveryAcknowledgments(1).event(0,"magic","operation-0"));
  rejects(()->new RecoveryAcknowledgments(1).event(0,"intent","other-0"));
  RecoveryAcknowledgments nested=new RecoveryAcknowledgments(1);nested.event(0,"intent","operation-0/create-stage");rejects(()->nested.event(1,"verified","operation-0/create-stage"));
  if(new RecoveryAcknowledgments(0).unresolved()!=0)throw new AssertionError();
  rejects(()->new RecoveryAcknowledgments(1).event(0,"intent","operation-0/unknown"));
  RecoveryAcknowledgments fabricated=new RecoveryAcknowledgments(1);fabricated.event(0,"intent","operation-0/create-stage");rejects(()->fabricated.event(1,"verified","operation-0"));
  fixtures();
  System.out.println("14 recovery acknowledgment checks passed");
 }

 private static void fixtures(){
  { // Exact phase/operation sequence from owned exported journal 56a1335a
   RecoveryAcknowledgments value=new RecoveryAcknowledgments(2);
   value.event(0,"intent","operation-0/directory-0");
   value.event(1,"returned","operation-0/directory-0");
   value.event(2,"observed","operation-0/directory-0");
   value.event(3,"intent","operation-0/directory-1");
   value.event(4,"returned","operation-0/directory-1");
   value.event(5,"observed","operation-0/directory-1");
   value.event(6,"intent","operation-0/create-stage");
   value.event(7,"returned","operation-0/create-stage");
   value.event(8,"observed","operation-0/create-stage");
   value.event(9,"intent","operation-0/write-stage");
   value.event(10,"returned","operation-0/write-stage");
   value.event(11,"intent","operation-0/publish-copy");
   value.event(12,"returned","operation-0/publish-copy");
   value.event(13,"observed","operation-0/publish-copy");
   value.event(14,"verified","operation-0");
   value.event(15,"intent","operation-1/create-stage");
   value.event(16,"returned","operation-1/create-stage");
   value.event(17,"observed","operation-1/create-stage");
   value.event(18,"intent","operation-1/write-stage");
   value.event(19,"returned","operation-1/write-stage");
   value.event(20,"intent","operation-1/publish-copy");
   value.event(21,"returned","operation-1/publish-copy");
   value.event(22,"observed","operation-1/publish-copy");
   value.event(23,"verified","operation-1");
   if(value.acknowledged()!=2||value.unresolved()!=0)throw new AssertionError();
  }
  { // Exact phase/operation sequence from owned exported journal 663ec98a
   RecoveryAcknowledgments value=new RecoveryAcknowledgments(2);
   value.event(0,"intent","operation-0/create-stage");
   value.event(1,"returned","operation-0/create-stage");
   value.event(2,"observed","operation-0/create-stage");
   value.event(3,"intent","operation-0/write-stage");
   value.event(4,"returned","operation-0/write-stage");
   value.event(5,"intent","operation-0/publish-copy");
   value.event(6,"returned","operation-0/publish-copy");
   value.event(7,"observed","operation-0/publish-copy");
   value.event(8,"destination-observed","operation-0");
   value.event(9,"verified","operation-0");
   value.event(10,"intent","operation-1/create-stage");
   value.event(11,"returned","operation-1/create-stage");
   value.event(12,"observed","operation-1/create-stage");
   value.event(13,"intent","operation-1/write-stage");
   value.event(14,"returned","operation-1/write-stage");
   if(value.acknowledged()!=1||value.unresolved()!=1)throw new AssertionError();
  }
 }
}
