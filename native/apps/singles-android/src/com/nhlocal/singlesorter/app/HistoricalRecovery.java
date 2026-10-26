package com.nhlocal.singlesorter.app;

import android.content.Context;
import java.io.IOException;
import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONObject;

/** Historical display only. Does not release write admission or mutate journals. */
final class HistoricalRecovery {
    private HistoricalRecovery(){}
    static Object latest(Context context,AtomicBoolean cancelled)throws Exception{
        int[] counts={0,0,0,0};boolean unreadable=false;
        try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
            lease.check();RetainedJournals.scan(context,cancelled::get,record->{if(record.attention){counts[0]++;counts[1]+=record.acknowledged;counts[2]+=record.unresolved;counts[3]+=record.duetUnresolved;}});
        }catch(IOException|org.json.JSONException|IllegalArgumentException|android.system.ErrnoException|SecurityException failure){unreadable=true;}
        JournalExportBounds.checkpoint(cancelled);if(!unreadable&&counts[0]==0)return JSONObject.NULL;
        return new JSONObject().put("recovery_notice",new JSONObject().put("status",unreadable?"unreadable":"attention").put("journal_count",Math.max(1,counts[0]))
            .put("acknowledged_operations",unreadable?JSONObject.NULL:counts[1]).put("unresolved_operations",unreadable?JSONObject.NULL:counts[2]).put("unresolved_duet_retirements",unreadable?JSONObject.NULL:counts[3]));
    }
}
