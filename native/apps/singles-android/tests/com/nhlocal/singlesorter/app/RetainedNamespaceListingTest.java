package com.nhlocal.singlesorter.app;
public final class RetainedNamespaceListingTest {
 private RetainedNamespaceListingTest(){}
 public static void main(String[] args){int count=0;RetainedNamespaceListing listing=new RetainedNamespaceListing("id",".singles-stage");if(!listing.absent(false,false))throw new AssertionError();count++;if(listing.absent(true,false)||listing.absent(false,true))throw new AssertionError();count++;
  listing.child("other","user.mp3");if(!listing.absent(false,false))throw new AssertionError();count++;listing.child("id","changed-name");if(listing.absent(false,false))throw new AssertionError();count++;
  RetainedNamespaceListing alias=new RetainedNamespaceListing("id",".singles-stage");alias.child("different-id",".SINGLES-STAGE");if(alias.absent(false,false))throw new AssertionError();count++;
  try{alias.child("different-id","other");throw new AssertionError();}catch(IllegalArgumentException expected){count++;}
  try{alias.child(null,"other");throw new AssertionError();}catch(IllegalArgumentException expected){count++;}
  System.out.println(count+" complete parent listing and unknown temporary absence checks passed");
 }
}
