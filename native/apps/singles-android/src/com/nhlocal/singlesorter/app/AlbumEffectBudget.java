package com.nhlocal.singlesorter.app;

import java.util.function.BooleanSupplier;
import org.json.JSONArray;

/** Producer phase counts, independent of byte-size estimates. Interrupted paths are prefixes. */
final class AlbumEffectBudget {
    private AlbumEffectBudget(){}
    static long setup(int components,boolean removeEmpty,boolean eligible){if(components<1||components>128)throw new IllegalArgumentException("ALBUM_COMPONENT_BOUND");return 2L+(eligible?3L*components:0)+(eligible&&removeEmpty?3:0);}
    static long copyFile(){return 12;} // start + create3 + write3 + publish3 + destination + verified
    static long copyDirectory(int descendantFiles,int descendantDirectories){if(descendantFiles<0||descendantDirectories<0||descendantFiles+descendantDirectories>2000)throw new IllegalArgumentException("ALBUM_SUBTREE_BOUND");return 9L+9L*descendantFiles+3L*descendantDirectories;}
    static long move(boolean rename){return rename?18:6;} // renamed: create/move/rename/publish/cleanup, three receipts each, plus three child receipts
    static long single(boolean copy,boolean rename,int components){if(components<1||components>128)throw new IllegalArgumentException("DESTINATION_LIMIT");return 3L*(components-1)+(copy?10:rename?17:5);}
    static long album(NativeAlbumPlan.Album album,AlbumSnapshotTree tree,BooleanSupplier cancelled)throws Exception{
        long count=setup(album.components().length(),album.removeEmpty,album.eligibleCount>0);
        for(NativeAlbumPlan.Child child:album.children){JournalExportBounds.checkpoint(cancelled);if(!child.eligible){count++;continue;}if(!album.copy){count+=move(!child.name.equals(child.source.name));continue;}if(!child.source.directory){count+=copyFile();continue;}
            int files=0,directories=0;for(AlbumSnapshotTree.Node member:tree.complete(child.source)){JournalExportBounds.checkpoint(cancelled);if(member==child.source)continue;if(member.directory)directories++;else files++;}count+=copyDirectory(files,directories);
        }JournalExportBounds.checkpoint(cancelled);return count;
    }
    static long single(NativeTransferPlan.Transfer transfer)throws Exception{JSONArray path=transfer.components();return single(transfer.copy,!transfer.displayName.equals(path.getString(path.length()-1)),path.length());}
}
