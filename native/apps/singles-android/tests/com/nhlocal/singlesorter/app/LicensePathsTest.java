package com.nhlocal.singlesorter.app;
public final class LicensePathsTest {
    public static void main(String[] args) {
        String[] good={"index.html","inventory.json","sources/taglib-2.1.1.tar.gz","rust/jni/LICENSE-MIT"};
        String[] bad={"","/index.html","../private","a/../b","a/./b","a//b","a/","a\\b","%2e%2e/a","https://evil","a?x","a#x","\u0000","é"};
        for(String s:good)if(!LicensePaths.safe(s))throw new AssertionError(s);
        for(String s:bad)if(LicensePaths.safe(s))throw new AssertionError(s);
        for(int bits=0;bits<16;bits++)if(LicensePaths.exportGrant((bits&1)!=0,(bits&2)!=0,(bits&4)!=0,(bits&8)!=0)!=(bits==15))throw new AssertionError("grant "+bits);
        System.out.println("34 license path/grant checks passed");
    }
}
