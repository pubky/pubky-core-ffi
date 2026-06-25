//! Android initialization for `rustls-platform-verifier`.
//!
//! pkarr's relay HTTP client (pulled in transitively, via reqwest 0.13's `rustls`
//! feature) uses `rustls-platform-verifier`. On Android that crate must be handed the
//! `JavaVM` + application `Context` once, before any TLS handshake, or the first
//! verification panics with "Expect rustls-platform-verifier to be initialized".
//! iOS/macOS use Security.framework and need no initialization, so this is Android-only.
//!
//! As of pubky 0.9.3 the SDK's own ICANN client no longer uses the platform verifier
//! (pubky-core#456); pkarr's relay client still does, so this init remains required until
//! pkarr#262 lands. Once it does, this file and the Android-only deps can be removed.
//!
//! The uniffi bindings load `libpubkycore.so` via JNA, so `JNI_OnLoad` never fires and
//! we can't capture the `JavaVM` automatically. Instead we expose a plain JNI entrypoint
//! that the Kotlin wrapper (`uniffi.pubkycore.RustlsInit`) binds via `external fun` after
//! `System.loadLibrary("pubkycore")`, passing the application `Context`.
//!
//! `init_with_env` is backed by a `OnceCell`, so this is idempotent and safe to call more
//! than once.

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_uniffi_pubkycore_RustlsInit_initPlatformVerifier<'caller>(
    mut env: jni::EnvUnowned<'caller>,
    _class: jni::objects::JClass<'caller>,
    context: jni::objects::JObject<'caller>,
) {
    // `with_env` upgrades the FFI-safe `EnvUnowned` into an `Env` and wraps the closure in
    // `catch_unwind`, so a JNI failure here can't unwind across the FFI boundary. We
    // deliberately discard the outcome: a failed init leaves the verifier uninitialized and
    // surfaces later as the original handshake error, which is the most we can do here.
    let _ = env.with_env(|env| -> Result<(), jni::errors::Error> {
        rustls_platform_verifier::android::init_with_env(env, context)
    });
}
