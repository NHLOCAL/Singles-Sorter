package com.nhlocal.singlesorter.app;
/** Every independently observed picker/provider property is necessary for retained local write scope. */
final class LocalWriteGrantPolicy {
    private LocalWriteGrantPolicy(){}
    static boolean accept(boolean tree,boolean authority,boolean systemPackage,boolean read,boolean write,boolean persistable){return tree&&authority&&systemPackage&&read&&write&&persistable;}
}
