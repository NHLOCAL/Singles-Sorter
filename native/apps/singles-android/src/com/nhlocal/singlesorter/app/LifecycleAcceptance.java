package com.nhlocal.singlesorter.app;
import java.util.function.BooleanSupplier;
/** Only short, I/O-free ownership publication belongs in this lifecycle boundary. */
final class LifecycleAcceptance {
    interface Start<T>{T run()throws Exception;}
    private LifecycleAcceptance(){}
    static <T>T accept(Object monitor,BooleanSupplier closed,Start<T> start)throws Exception{
        synchronized(monitor){if(closed.getAsBoolean())throw new IllegalStateException("HOST_CLOSED");return start.run();}
    }
}
