package com.nhlocal.singlesorter.app;

/** One local notice subtree, never a caller-selected filesystem path. */
final class LicensePaths {
    private LicensePaths() {}
    static boolean safe(String path) {
        if(path==null||path.length()==0||path.length()>512)return false;
        for(String part:path.split("/",-1)) {
            if(part.isEmpty()||part.equals(".")||part.equals("..")||!part.matches("[A-Za-z0-9._-]+"))return false;
        }
        return true;
    }
}
