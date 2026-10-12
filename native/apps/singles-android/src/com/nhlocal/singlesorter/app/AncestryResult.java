package com.nhlocal.singlesorter.app;
import java.io.IOException;
/** A missing or malformed provider response is never a negative ancestry fact. */
final class AncestryResult {
 private AncestryResult(){}
 interface Query {Object call()throws Exception;}
 static boolean query(Query query)throws Exception{Object value=query.call();if(!(value instanceof Boolean))throw new IOException("ANCESTRY_UNAVAILABLE");return (Boolean)value;}
}
