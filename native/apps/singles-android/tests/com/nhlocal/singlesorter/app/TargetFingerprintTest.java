package com.nhlocal.singlesorter.app;
public final class TargetFingerprintTest {
    public static void main(String[] args){
        TargetFingerprint original=new TargetFingerprint("id","name",false,"10",44L,"parent");
        if(!original.same(new TargetFingerprint("id","name",false,"10",44L,"parent")))throw new AssertionError("equal observation");
        TargetFingerprint[] changed={new TargetFingerprint("new","name",false,"10",44L,"parent"),new TargetFingerprint("id","new",false,"10",44L,"parent"),new TargetFingerprint("id","name",true,"10",44L,"parent"),new TargetFingerprint("id","name",false,"11",44L,"parent"),new TargetFingerprint("id","name",false,"10",45L,"parent"),new TargetFingerprint("id","name",false,"10",44L,"other"),new TargetFingerprint("id","name",false,null,44L,"parent")};
        for(TargetFingerprint value:changed)if(original.same(value))throw new AssertionError("changed observation accepted");
        if(!new TargetFingerprint("id","dir",true,"10",0L,null).same(new TargetFingerprint("id","dir",true,"10",null,null)))throw new AssertionError("directory size is not a file binding");
        if(!original.sameStructure(new TargetFingerprint("id","name",false,"11",45L,"parent")))throw new AssertionError("owned metadata update");
        for(int index:new int[]{0,1,2,5})if(original.sameStructure(changed[index]))throw new AssertionError("structure silently absorbed");
        System.out.println("14 target observation checks passed");
    }
}
