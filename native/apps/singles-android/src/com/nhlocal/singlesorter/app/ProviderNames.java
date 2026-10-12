package com.nhlocal.singlesorter.app;
import java.nio.charset.StandardCharsets;
final class ProviderNames {
    private ProviderNames() {}
    static void require(String name){
        if(name==null||name.isEmpty()||name.equals(".")||name.equals("..")||name.indexOf('/')>=0||name.indexOf('\\')>=0||name.indexOf('\u0000')>=0||name.getBytes(StandardCharsets.UTF_8).length>255)throw new IllegalArgumentException("PROVIDER_NAME");
    }
}
