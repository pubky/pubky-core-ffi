package org.pubky.consumer

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pubkycore.RustlsInit
import uniffi.pubkycore.generateSecretKey
import uniffi.pubkycore.get

@RunWith(AndroidJUnit4::class)
class NativeSmokeTest {
    @Test
    fun nativeLibraryAndPlatformVerifierWork() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        RustlsInit.ensure(context)
        RustlsInit.ensure(context)
        assertEquals("false", generateSecretKey().first())
        val result = get("https://example.com/")
        assertEquals(result.joinToString(), "false", result.first())
        assertTrue(result[1].contains("Example Domain"))
    }
}
