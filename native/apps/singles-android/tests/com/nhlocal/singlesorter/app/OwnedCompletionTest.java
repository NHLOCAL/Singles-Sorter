package com.nhlocal.singlesorter.app;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicInteger;
public final class OwnedCompletionTest {
    private static int checks;
    private static void check(boolean value){if(!value)throw new AssertionError();checks++;}
    public static void main(String[] args)throws Exception {
        Object lock=new Object();boolean[] busy={true};AtomicInteger release=new AtomicInteger(),published=new AtomicInteger();
        OwnedCompletion first=new OwnedCompletion(lock,()->{check(busy[0]);busy[0]=false;release.incrementAndGet();});
        first.finish(()->{check(!busy[0]);published.incrementAndGet();busy[0]=true;});
        check(busy[0]);first.finish(()->{throw new AssertionError("duplicate publication");});check(busy[0]);check(release.get()==1);check(published.get()==1);
        CountDownLatch ready=new CountDownLatch(1),go=new CountDownLatch(1);
        Thread delayed=new Thread(()->{ready.countDown();try{go.await();}catch(InterruptedException error){throw new AssertionError(error);}first.finish(()->{throw new AssertionError("late publication");});});
        delayed.start();ready.await();go.countDown();delayed.join();check(busy[0]);check(release.get()==1);
        Object oldSignal=new Object(),newSignal=new Object();Object[] signal={oldSignal};
        OwnedCompletion documents=new OwnedCompletion(lock,()->{check(signal[0]==oldSignal);signal[0]=null;busy[0]=false;});
        documents.finish(()->{signal[0]=newSignal;busy[0]=true;});documents.finish(()->{throw new AssertionError();});check(signal[0]==newSignal);check(busy[0]);
        System.out.println(checks+" delayed/reentrant completion checks passed");
    }
}
