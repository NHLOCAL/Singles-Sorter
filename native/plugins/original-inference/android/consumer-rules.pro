# Keep JNI symbol-based entry points if a future host enables R8/ProGuard.
-keep class com.nhlocal.singlesorter.inference.OriginalInference {
    native <methods>;
}
