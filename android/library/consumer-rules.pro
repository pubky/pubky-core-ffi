# UniFFI uses JNA method names to resolve native symbols at runtime.
-keep class uniffi.pubkycore.** { *; }

# rustls-platform-verifier calls these classes and methods through JNI.
-keep class org.rustls.platformverifier.** { *; }
