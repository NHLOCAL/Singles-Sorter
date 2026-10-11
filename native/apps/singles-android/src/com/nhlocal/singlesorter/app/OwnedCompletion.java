package com.nhlocal.singlesorter.app;

/** Release once, before publishing, under the same lock used to accept new work.
 * A delayed/duplicate old completion cannot release a successor's ownership.
 */
final class OwnedCompletion {
    private final Object lock;private final Runnable release;private boolean completed;
    OwnedCompletion(Object lock,Runnable release){this.lock=lock;this.release=release;}
    void finish(Runnable publication){synchronized(lock){if(completed)return;completed=true;release.run();publication.run();}}
}
