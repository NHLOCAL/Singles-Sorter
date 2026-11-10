package com.nhlocal.singlesorter.app;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
public final class LifecycleIoTest {
    private LifecycleIoTest(){}
    public static void main(String[] args)throws Exception{
        Object monitor=new Object();boolean[] called={false};
        synchronized(monitor){try{LifecycleIo.run(monitor,()->{called[0]=true;return null;});throw new AssertionError();}catch(IllegalStateException expected){}}
        if(called[0])throw new AssertionError("Provider called under lifecycle lock");
        CountDownLatch providerEntered=new CountDownLatch(1),releaseProvider=new CountDownLatch(1),closeFinished=new CountDownLatch(1);AtomicReference<Throwable> failure=new AtomicReference<>();
        Thread worker=new Thread(()->{try{LifecycleIo.run(monitor,()->{providerEntered.countDown();if(!releaseProvider.await(5,TimeUnit.SECONDS))throw new AssertionError("Provider timeout");return null;});}catch(Throwable e){failure.set(e);}});
        worker.start();if(!providerEntered.await(2,TimeUnit.SECONDS))throw new AssertionError("Provider did not start");
        Thread close=new Thread(()->{synchronized(monitor){closeFinished.countDown();}});close.start();
        try{if(!closeFinished.await(1,TimeUnit.SECONDS))throw new AssertionError("Close/cancel blocked behind provider");}finally{releaseProvider.countDown();worker.join(2000);close.join(2000);}
        if(worker.isAlive()||close.isAlive()||failure.get()!=null)throw new AssertionError(failure.get());
        System.out.println("4 delayed provider lifecycle checks passed");
    }
}
