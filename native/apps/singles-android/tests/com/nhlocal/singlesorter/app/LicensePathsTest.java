package com.nhlocal.singlesorter.app;
public final class LicensePathsTest {
    public static void main(String[] args) {
        String[] good={"index.html","inventory.json","sources/taglib-2.1.1.tar.gz","rust/jni/LICENSE-MIT"};
        String[] bad={"","/index.html","../private","a/../b","a/./b","a//b","a/","a\\b","%2e%2e/a","https://evil","a?x","a#x","\u0000","é"};
        for(String s:good)if(!LicensePaths.safe(s))throw new AssertionError(s);
        for(String s:bad)if(LicensePaths.safe(s))throw new AssertionError(s);
        System.out.println("18 license path checks passed");
    }
}
