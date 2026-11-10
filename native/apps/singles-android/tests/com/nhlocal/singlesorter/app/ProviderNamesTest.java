package com.nhlocal.singlesorter.app;
public final class ProviderNamesTest {
    public static void main(String[] args){
        for(String name:new String[]{"אברהם פריד.wav","album",".owned-stage.bin","name (1).mp3"})ProviderNames.require(name);
        for(String name:new String[]{"",".","..","a/b","a\\b","a\u0000b",new String(new char[256]).replace('\u0000','a')}){
            try{ProviderNames.require(name);throw new AssertionError("accepted invalid name");}catch(IllegalArgumentException expected){}
        }
        try{ProviderNames.require(null);throw new AssertionError("accepted null");}catch(IllegalArgumentException expected){}
        System.out.println("12 provider component checks passed");
    }
}
