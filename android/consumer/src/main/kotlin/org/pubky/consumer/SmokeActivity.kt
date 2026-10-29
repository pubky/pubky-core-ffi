package org.pubky.consumer

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import uniffi.pubkycore.RustlsInit
import uniffi.pubkycore.generateSecretKey

class SmokeActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        RustlsInit.ensure(this)
        check(generateSecretKey().first() == "false")
        setContentView(TextView(this).apply { text = "Pubky native library loaded" })
    }
}
