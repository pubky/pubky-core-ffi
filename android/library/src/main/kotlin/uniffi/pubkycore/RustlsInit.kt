package uniffi.pubkycore

import android.content.Context

/** Initializes Android certificate verification before the first Pubky network call. */
object RustlsInit {
    @JvmStatic
    private external fun initPlatformVerifier(context: Context)

    @Volatile
    private var initialized = false

    @JvmStatic
    @Synchronized
    fun ensure(context: Context) {
        if (initialized) return
        // UniFFI loads through JNA, so bind the explicit JNI initializer as well.
        System.loadLibrary("pubkycore")
        initPlatformVerifier(context.applicationContext)
        initialized = true
    }
}
