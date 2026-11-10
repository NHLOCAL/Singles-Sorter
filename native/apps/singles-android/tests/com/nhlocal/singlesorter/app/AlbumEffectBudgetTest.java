package com.nhlocal.singlesorter.app;

public final class AlbumEffectBudgetTest {
    private static int checks;private AlbumEffectBudgetTest(){}private static void check(boolean ok){if(!ok)throw new AssertionError();checks++;}
    public static void main(String[] args){
        check(AlbumEffectBudget.move(true)==18);check(AlbumEffectBudget.move(false)==6);check(AlbumEffectBudget.copyFile()==12);check(AlbumEffectBudget.copyDirectory(0,0)==9);check(AlbumEffectBudget.copyDirectory(2,2)==33);
        check(AlbumEffectBudget.setup(2,true,true)==11);check(AlbumEffectBudget.setup(2,false,true)==8);check(AlbumEffectBudget.setup(2,true,false)==2);
        check(AlbumEffectBudget.single(true,false,1)==10);check(AlbumEffectBudget.single(false,false,1)==5);check(AlbumEffectBudget.single(false,true,1)==17);check(AlbumEffectBudget.single(false,true,3)==23);
        check(500L*AlbumEffectBudget.move(true)+AlbumEffectBudget.setup(2,true,true)>9000);check(500L*14+20+2*3<9000);
        System.out.println(checks+" exact Album and Single phase-budget checks passed");
    }
}
