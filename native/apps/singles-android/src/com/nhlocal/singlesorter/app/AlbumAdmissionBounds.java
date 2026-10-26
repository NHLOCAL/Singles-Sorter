package com.nhlocal.singlesorter.app;

import java.nio.charset.StandardCharsets;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;

/** Conservative whole mixed-job budget before the first provider effect. */
final class AlbumAdmissionBounds {
    private AlbumAdmissionBounds(){}
    static void require(RetainedPlan retained,NativeTransferPlan singles,NativeAlbumPlan albums,BooleanSupplier cancelled)throws Exception{
        if(albums.operations.isEmpty())return;long events=0,estimated=4096;String target=retained.planCopy().getJSONObject("request").getJSONObject("target").getJSONObject("AndroidDocument").getString("tree_uri");
        for(NativeTransferPlan.Transfer single:singles.operations){JournalExportBounds.checkpoint(cancelled);long count=AlbumEffectBudget.single(single);events+=count;long uri=uriBound(single.source().getJSONObject("AndroidDocument").toString(),target,single.components());estimated+=count*(512+3*uri);}
        for(NativeAlbumPlan.Album album:albums.operations){JournalExportBounds.checkpoint(cancelled);long count=AlbumEffectBudget.album(album,albums.tree,cancelled),max=album.source.parent==null&&retained.albumRootParentCopy()!=null?AlbumRootParent.uriBudget(retained.albumRootParentCopy()):0;
            for(AlbumSnapshotTree.Node node:albums.tree.complete(album.source)){JournalExportBounds.checkpoint(cancelled);max=Math.max(max,uriBound(FixRecoveryEvidence.uri(node.entry()),target,album.components()));}
            events+=count;estimated+=count*(512+3*max);if(events>9000||estimated>7L*1024*1024)throw new IllegalArgumentException("ALBUM_JOB_BUDGET");
        }
        JournalExportBounds.checkpoint(cancelled);
    }
    private static long uriBound(String source,String target,JSONArray destination)throws Exception{
        long result=source.getBytes(StandardCharsets.UTF_8).length+target.getBytes(StandardCharsets.UTF_8).length+1024;for(int i=0;i<destination.length();i++)result+=destination.getString(i).getBytes(StandardCharsets.UTF_8).length*3L+3;
        if(result>8192)throw new IllegalArgumentException("ALBUM_URI_BUDGET");return result;
    }
}
