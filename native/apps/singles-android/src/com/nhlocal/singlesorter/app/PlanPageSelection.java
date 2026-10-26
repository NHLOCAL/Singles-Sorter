package com.nhlocal.singlesorter.app;

/** Stable display-only filtering; original plan rows and operation IDs remain intact. */
final class PlanPageSelection {
    final int[] indices;final int total;final int skipped;
    private PlanPageSelection(int[] indices,int total,int skipped){this.indices=indices;this.total=total;this.skipped=skipped;}
    static PlanPageSelection select(boolean[] developerOnly,boolean[] skippedRows,boolean developerMode,int offset,int limit){
        return select(developerOnly,skippedRows,developerMode,true,offset,limit);
    }
    static PlanPageSelection select(boolean[] developerOnly,boolean[] skippedRows,boolean developerMode,boolean showInactive,int offset,int limit){
        if(developerOnly.length!=skippedRows.length||offset<0||limit<1||limit>200)throw new IllegalArgumentException("PAGE_LIMIT");
        int[] selected=new int[Math.min(limit,developerOnly.length)];int count=0,total=0,skipped=0;
        for(int n=0;n<developerOnly.length;n++){
            if(!developerMode&&developerOnly[n])continue;
            if(skippedRows[n])skipped++;
            if(!showInactive&&skippedRows[n])continue;
            if(total>=offset&&count<limit)selected[count++]=n;
            total++;
        }
        return new PlanPageSelection(java.util.Arrays.copyOf(selected,count),total,skipped);
    }
}
