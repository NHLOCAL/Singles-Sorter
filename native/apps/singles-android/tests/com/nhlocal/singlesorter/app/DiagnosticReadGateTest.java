package com.nhlocal.singlesorter.app;
public final class DiagnosticReadGateTest {
    private DiagnosticReadGateTest(){}
    public static void main(String[] args){
        DiagnosticReadGate gate=new DiagnosticReadGate();
        try{gate.acquire(true);throw new AssertionError();}catch(IllegalStateException expected){}
        DiagnosticReadGate.Lease first=gate.acquire(false);first.check();
        if(!gate.held())throw new AssertionError();
        try{gate.acquire(false);throw new AssertionError();}catch(IllegalStateException expected){}
        first.close();if(gate.held())throw new AssertionError();
        DiagnosticReadGate.Lease second=gate.acquire(false);first.close();second.check();
        if(!gate.held())throw new AssertionError();
        second.close();if(gate.held())throw new AssertionError();
        System.out.println("7 diagnostic ownership checks passed");
    }
}
