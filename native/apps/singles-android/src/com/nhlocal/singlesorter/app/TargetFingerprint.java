package com.nhlocal.singlesorter.app;
import java.util.Objects;
/** Equality of provider observations only, not a held-object identity guarantee. */
final class TargetFingerprint {
    final String id,name,parent,modified;final boolean directory;final Long size;
    TargetFingerprint(String id,String name,boolean directory,String modified,Long size,String parent){
        this.id=Objects.requireNonNull(id);this.name=Objects.requireNonNull(name);this.directory=directory;this.modified=modified;this.size=directory?null:size;this.parent=parent;
    }
    boolean sameStructure(TargetFingerprint other){return id.equals(other.id)&&name.equals(other.name)&&directory==other.directory&&Objects.equals(parent,other.parent);}
    boolean same(TargetFingerprint other){return sameStructure(other)&&Objects.equals(modified,other.modified)&&Objects.equals(size,other.size);}
}
