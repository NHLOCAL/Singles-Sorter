package com.nhlocal.singlesorter.app;
import java.util.Arrays;
public final class PlanPageSelectionTest {
    private PlanPageSelectionTest() {}
    private static void check(boolean condition){if(!condition)throw new AssertionError("page filtering");}
    public static void main(String[] args){
        boolean[] hidden={true,false,true,false,false},skipped={false,true,false,false,true};
        PlanPageSelection first=PlanPageSelection.select(hidden,skipped,false,0,2);
        check(Arrays.equals(first.indices,new int[]{1,3}));check(first.total==3&&first.skipped==2);
        PlanPageSelection second=PlanPageSelection.select(hidden,skipped,false,2,2);
        check(Arrays.equals(second.indices,new int[]{4}));check(second.total==3&&second.skipped==2);
        PlanPageSelection developer=PlanPageSelection.select(hidden,skipped,true,0,200);
        check(Arrays.equals(developer.indices,new int[]{0,1,2,3,4}));check(developer.total==5&&developer.skipped==2);
        check(PlanPageSelection.select(hidden,skipped,false,Integer.MAX_VALUE,200).indices.length==0);
        check(PlanPageSelection.select(new boolean[0],new boolean[0],false,0,1).total==0);
        for(int[] invalid:new int[][]{{-1,1},{0,0},{0,201}}){try{PlanPageSelection.select(hidden,skipped,false,invalid[0],invalid[1]);throw new AssertionError("invalid page");}catch(IllegalArgumentException expected){}}
        System.out.println("11 display pagination checks passed");
    }
}
