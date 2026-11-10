package com.nhlocal.singlesorter.app;
public final class LocalWriteGrantPolicyTest {
    private LocalWriteGrantPolicyTest(){}
    public static void main(String[] args){
        for(int mask=0;mask<64;mask++){boolean actual=LocalWriteGrantPolicy.accept((mask&1)!=0,(mask&2)!=0,(mask&4)!=0,(mask&8)!=0,(mask&16)!=0,(mask&32)!=0);if(actual!=(mask==63))throw new AssertionError("grant mask "+mask);}
        System.out.println("64 local write grant combinations passed");
    }
}
