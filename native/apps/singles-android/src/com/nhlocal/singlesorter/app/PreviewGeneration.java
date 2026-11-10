package com.nhlocal.singlesorter.app;
import java.io.IOException;
/** Process-only preview invalidation; restart retains no old in-memory preview authority. */
final class PreviewGeneration {
 static final PreviewGeneration PROCESS=new PreviewGeneration(0);
 private long value;private boolean exhausted, accepting;
 PreviewGeneration(long initial){if(initial<0)throw new IllegalArgumentException("generation");value=initial;}
 synchronized long capture()throws IOException{if(exhausted||accepting)throw new IOException("NEW_PREVIEW_REQUIRED");return value;}
 synchronized void require(long captured)throws IOException{if(exhausted||accepting||captured!=value)throw new IOException("NEW_PREVIEW_REQUIRED");}
 synchronized void beginAcceptance()throws IOException{if(accepting)throw new IOException("ACCEPTANCE_ACTIVE");invalidate();accepting=true;}
 synchronized void endAcceptance(){accepting=false;}
 synchronized void invalidate()throws IOException{if(exhausted||value==Long.MAX_VALUE){exhausted=true;throw new IOException("PREVIEW_GENERATION_EXHAUSTED");}value++;}
}
