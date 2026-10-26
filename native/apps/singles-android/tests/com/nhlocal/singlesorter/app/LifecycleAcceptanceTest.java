package com.nhlocal.singlesorter.app;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
public final class LifecycleAcceptanceTest {
    private LifecycleAcceptanceTest(){}
    public static void main(String[] args)throws Exception{
        Object monitor=new Object();boolean[] closed={false},started={false};CountDownLatch queued=new CountDownLatch(1),release=new CountDownLatch(1);AtomicReference<Throwable> unexpected=new AtomicReference<>();
        Thread worker=new Thread(()->{try{queued.countDown();if(!release.await(3,TimeUnit.SECONDS))throw new AssertionError();LifecycleAcceptance.accept(monitor,()->closed[0],()->{started[0]=true;return null;});unexpected.set(new AssertionError("Accepted after close"));}catch(IllegalStateException expected){if(!"HOST_CLOSED".equals(expected.getMessage()))unexpected.set(expected);}catch(Throwable error){unexpected.set(error);}});
        worker.start();if(!queued.await(2,TimeUnit.SECONDS))throw new AssertionError();synchronized(monitor){closed[0]=true;}release.countDown();worker.join(2000);
        if(worker.isAlive()||started[0]||unexpected.get()!=null)throw new AssertionError(unexpected.get());
        Object open=new Object();if(!LifecycleAcceptance.accept(open,()->false,()->Thread.holdsLock(open)))throw new AssertionError();
        System.out.println("3 queued-start lifecycle checks passed");
    }
}
