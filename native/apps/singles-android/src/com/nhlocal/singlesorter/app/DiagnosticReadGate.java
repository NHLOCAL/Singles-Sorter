package com.nhlocal.singlesorter.app;
/** Short ownership decisions only. File/provider I/O never runs under this monitor. */
final class DiagnosticReadGate {
    private Lease owner;
    synchronized Lease acquire(boolean executionActive){
        if(executionActive||owner!=null)throw new IllegalStateException("EXECUTION_ACTIVE");
        owner=new Lease(this);return owner;
    }
    synchronized boolean held(){return owner!=null;}
    private synchronized void release(Lease lease){if(owner==lease)owner=null;}
    static final class Lease implements AutoCloseable {
        private final DiagnosticReadGate gate;
        private Lease(DiagnosticReadGate gate){this.gate=gate;}
        void check(){synchronized(gate){if(gate.owner!=this)throw new IllegalStateException("DIAGNOSTIC_OWNER");}}
        @Override public void close(){gate.release(this);}
    }
}
