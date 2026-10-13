package com.nhlocal.singlesorter.app;
/** Provider/file I/O may never run while holding the monitor needed by Activity close/cancel. */
final class LifecycleIo {
    interface Action<T>{T run()throws Exception;}
    private LifecycleIo(){}
    static <T>T run(Object lifecycleMonitor,Action<T> action)throws Exception{
        if(Thread.holdsLock(lifecycleMonitor))throw new IllegalStateException("LIFECYCLE_IO_LOCK");
        return action.run();
    }
}
