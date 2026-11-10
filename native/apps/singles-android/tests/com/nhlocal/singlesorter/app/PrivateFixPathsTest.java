package com.nhlocal.singlesorter.app;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;

public final class PrivateFixPathsTest {
    private static int checks;
    private static final String ID="00000000-0000-0000-0000-000000000001";
    private static void check(boolean value){checks++;if(!value)throw new AssertionError();}
    private interface Action {void run()throws Exception;}
    private static void refuses(Action action)throws Exception{checks++;try{action.run();throw new AssertionError("accepted");}catch(IOException expected){}}
    public static void main(String[] args)throws Exception{
        Path owned=Files.createTempDirectory("private-fix-paths-");
        try{
            Path real=Files.createDirectory(owned.resolve("real"));
            Path alias=Files.createSymbolicLink(owned.resolve("alias"),real);
            File root=PrivateFixPaths.trustedRoot(alias.toFile());check(root.equals(real.toFile().getCanonicalFile()));
            File child=PrivateFixPaths.create(root,ID);check(child.getCanonicalFile().equals(child.getAbsoluteFile()));check(child.getParentFile().equals(root));
            Files.write(child.toPath().resolve("owned-marker"),new byte[]{1,2,3});
            refuses(()->PrivateFixPaths.create(root,ID));check(Files.readAllBytes(child.toPath().resolve("owned-marker")).length==3);
            Path foreign=Files.createDirectory(owned.resolve("foreign"));Files.write(foreign.resolve("marker"),new byte[]{7});
            String second="00000000-0000-0000-0000-000000000002";Files.createSymbolicLink(real.resolve("fix-"+second),foreign);
            refuses(()->PrivateFixPaths.create(root,second));check(Files.readAllBytes(foreign.resolve("marker"))[0]==7);
            String third="00000000-0000-0000-0000-000000000003";Files.write(real.resolve("fix-"+third),new byte[]{9});
            refuses(()->PrivateFixPaths.create(root,third));check(Files.readAllBytes(real.resolve("fix-"+third))[0]==9);
            refuses(()->PrivateFixPaths.create(root,"../escape"));refuses(()->PrivateFixPaths.create(root,null));
            refuses(()->PrivateFixPaths.create(alias.toFile(),"00000000-0000-0000-0000-000000000004"));
            refuses(()->PrivateFixPaths.trustedRoot(owned.resolve("missing").toFile()));
            refuses(()->PrivateFixPaths.trustedRoot(real.resolve("fix-"+third).toFile()));
            check(!Files.exists(owned.resolve("escape")));
        }finally{
            try(java.util.stream.Stream<Path> paths=Files.walk(owned)){
                for(Path p:paths.sorted(Comparator.reverseOrder()).toArray(Path[]::new))Files.delete(p);
            }
        }
        System.out.println(checks+" trusted cache alias and exclusive Fix child checks passed");
    }
}
