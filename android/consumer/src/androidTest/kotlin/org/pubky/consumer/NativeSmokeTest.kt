package org.pubky.consumer

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pubkycore.RustlsInit
import uniffi.pubkycore.generateSecretKey
import uniffi.pubkycore.GrantAuthFlowConfig
import uniffi.pubkycore.PubkyCoreException
import uniffi.pubkycore.startGrantAuthFlowWithConfig
import uniffi.pubkycore.pollGrantAuthFlow
import uniffi.pubkycore.cancelGrantAuthFlow

@RunWith(AndroidJUnit4::class)
class NativeSmokeTest {
    @Test(timeout = 30000)
    fun nativeLibraryAndPlatformVerifierWork() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        RustlsInit.ensure(context)
        RustlsInit.ensure(context)
        assertEquals("false", generateSecretKey().first())
        // Polling a nonexistent channel on example.com performs only HTTPS GETs.
        // Its HTTP 404 proves the native client completed TLS; a transport or
        // certificate error must fail the test rather than count as success.
        startGrantAuthFlowWithConfig(GrantAuthFlowConfig(
            capabilities = "/pub/:r", clientId = "org.pubky.consumer.smoke",
            homeserver = null, signupToken = null, relay = "https://example.com/inbox",
            clientSecret = null, clientKeySecret = null, approvalFormat = null,
            xSource = null, xSuccess = null, xError = null, xCancel = null,
        ))
        try {
            repeat(200) {
                try {
                    pollGrantAuthFlow()
                } catch (error: PubkyCoreException.Server) {
                    assertEquals(404, error.status.toInt())
                    return
                }
                Thread.sleep(100)
            }
            fail("Native HTTPS request did not finish")
        } finally {
            cancelGrantAuthFlow()
        }
    }
}
