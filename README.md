# Pubky Core Mobile SDK

The Pubky Core Mobile SDK provides native bindings for iOS and Android platforms to interact with Pubky. This SDK allows you to perform operations like publishing content, retrieving data and managing authentication.

## String Contracts

These output formats are intentional and relied upon by downstream consumers
(react-native-pubky, Pubky Ring, Bitkit). Do not change them without
coordinating a migration across those apps.

- **Public keys are always bare z-base32** (52 chars, no prefix) in every
  output: `public_key` fields, the session `pubky` field, `get_homeserver`,
  and the `publish`/`publish_https` return values. The underlying
  `pubky` crate (0.16.x) renders `PublicKey::to_string()` as
  `pubky<z32>`, so all output sites must use `.z32()` instead — pubky's own
  storage URL parser rejects `pubky://pubky<z32>/...`, and downstream apps
  build `pubky://<key>/...` URLs from these values. Inputs accept either form.
- **`uri` fields** use the pkarr URI form `pk:<z32>`.
- **Response vectors** are `[error, data]` where `error` is the string
  `"true"` or `"false"`.
- **`parse_auth_url`** returns `relay`, `capabilities`, `secret`, plus
  `kind` (`"signin"` or `"signup"`; legacy `pubkyauth:///?...` URLs are
  `"signin"`) and, for signup links, optional `homeserver` (bare z32) and
  `signup_token` fields.
- **Authentication mirrors pubky 0.16.** Grant auth is exposed through
  `sign_up_grant`, `sign_in_grant`, `start_grant_auth_flow`, and
  `await_grant_auth_approval`; these return `grant_secret` in session JSON.
  The unqualified `sign_up`, `sign_in`, `start_auth_flow`, and
  `await_auth_approval` names are Grant aliases for downstream migration.
- **Signer approval can be narrowed.** `auth` approves every capability in an
  auth request, while `auth_with_capabilities` accepts the user-approved
  comma-separated `scope:actions` subset. An empty string approves no
  capabilities; the SDK rejects approvals that exceed the request.
- **Legacy cookie auth remains available** through `sign_up_cookie`,
  `sign_in_cookie`, `start_cookie_auth_flow`, and
  `await_cookie_auth_approval`; these return `session_secret` in session JSON.
  Cookie auth is deprecated upstream, but it is still exposed here because this
  crate is a low-level binding layer.
- **Session-token APIs accept either strategy.** `sign_out`,
  `revalidate_session`, `put_with_session`, and `delete_with_session` call
  `Pubky::restore_session`, so they accept a Grant `grant_secret` or a legacy
  cookie `session_secret`.
- **Grant management requires a root-capability session.** `list_grants`
  returns the account's active grants and `revoke_grant` revokes a grant by
  id, invalidating all sessions minted from it.
- **New Pubky 0.16 APIs use typed native errors.** Existing functions keep the
  `[error, data]` vector contract. The additive storage, event, lock, client
  configuration, blocking-signin, and advanced Grant-flow APIs return native
  `Result` values with `PubkyCoreError` variants instead.

## Pubky 0.16 Client Bindings

The high-level Pubky 0.16 client surface is available to generated Swift,
Kotlin, and Python bindings:

- Client configuration: request/read timeouts, connection pooling, bounded
  error bodies, user-agent suffixes, mainnet, and custom-host testnet.
- Storage: public and authenticated reads, public and private paths, raw bytes,
  existence checks, metadata, paginated listings, writes, and deletes.
- Authentication: Grant signin and signup flows, relay and callback
  configuration, save/restore, non-blocking polling, cancellation, blocking
  signin, session revalidation, Grant management, signed approvals, scoped
  encryption keys, and custom offline-verifiable proofs.
- Events: historical and live streams, multiple users, cursors, path filters,
  private authenticated streams, reverse order, limits, and cancellation.
- Locks: acquire, inspect, refresh, write, delete, and unlock.
- Structured failures: transport, server status, validation, JSON decoding,
  PKARR retryability, parsing, authentication expiry, build, and binding-state
  errors.

See [BINDING_PARITY.md](BINDING_PARITY.md) for the SDK-to-FFI mapping and the
small set of Rust-specific low-level primitives intentionally kept internal.

## Building the SDK

### To build both iOS and Android bindings:
```
./build.sh all
```

### To build only iOS bindings:
```
./build.sh ios
```

### To build only Android bindings:
```
./build.sh android
```

The Android build is pinned to the NDK version in `.ndk-version` (currently
r29) and requires NDK r28 or newer. Install the pinned version with
`sdkmanager "ndk;$(cat .ndk-version)"`. NDK r28+ emits ELF `LOAD` segments
compatible with Android's 16 KB page size by default. Maven publication
verification inspects the packaged AAR and fails if any 64-bit shared library
has a `LOAD` alignment below `0x4000`.

To verify already-built Android bindings without rebuilding them:

```
ANDROID_NDK_HOME="$ANDROID_SDK_ROOT/ndk/$(cat .ndk-version)" \
  ./scripts/verify_android_page_size.sh
```

### To build only Python bindings:
```
./build.sh python
```

### Release artifacts

Generated native libraries are not stored in Git. A release matching the Cargo
package version, such as `v0.4.0`, distributes:

- `org.pubky:pubky-core-android:<version>` to the repository's GitHub Packages
  Maven registry. Its AAR contains the compiled Kotlin bindings, rustls
  platform-verifier support, and JNI libraries for all four Android ABIs.
- `pubky-core-ffi-ios.zip` and `PubkyCore.xcframework.zip` through GitHub
  Releases. The first archive supports manual integration. The second is the
  binary artifact consumed by the repository's Swift package. Both support iOS
  devices and Apple Silicon simulators.
- Platform-specific Python wheels, built and tested by `Python wheels` CI.
  PyPI publication is enabled after project ownership and Trusted Publishing
  are configured; workflow artifacts can be installed directly beforehand.

A manually dispatched or pull-request workflow produces a local Maven
repository and iOS archives as temporary validation artifacts. Tagged builds
publish Android with the workflow's `GITHUB_TOKEN`; no repository publishing
secrets are required. The iOS release archives are built and uploaded locally
because rebuilding an XCFramework changes its SwiftPM checksum. Local build
output and `dist/` are ignored; run
`scripts/verify_source_only.sh` before committing to ensure a native artifact
is not accidentally added back to the repository.

See `RELEASE.md` for the release sequence. The exact locally checksummed iOS
archive must be uploaded; a CI rebuild is suitable for validation only.

For Python installation and the supported wheel platforms, see
[`bindings/python/README.md`](bindings/python/README.md).

## Run Tests:
```
cargo test -- --test-threads=1
```

## iOS Integration

### Installation
For Swift Package Manager, add
`https://github.com/pubky/pubky-core-ffi` and select an exact released version.
The `PubkyCore` product downloads and verifies the corresponding XCFramework;
applications can then `import PubkyCore`.

For manual integration:

1. Build the iOS bindings or download `pubky-core-ffi-ios.zip` from a pinned
   GitHub Release.

2. Add the XCFramework to your Xcode project:

   - Drag bindings/ios/PubkyCore.xcframework into your Xcode project
     Ensure "Copy items if needed" is checked
     Add the framework to your target


3. Copy the Swift bindings:

   - Add bindings/ios/pubkycore.swift to your project

### Basic Usage
```swift
import PubkyCore

class PubkyManager(context: Context) {
    // Generate a new secret key
    func generateNewAccount() throws -> String {
        let result = try generateSecretKey()
        guard let jsonData = result[1].data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
              let secretKey = json["secret_key"] as? String else {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: "Failed to parse response"])
        }
        return secretKey
    }
    
    // Get a signup token
    func getSignupToken(homeserverPubky: String, adminPassword: String) async throws -> String {
        let result = try getSignupToken(homeserverPubky: homeserverPubky, adminPassword: adminPassword)
        if result[0] == "true" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Sign up with a homeserver (with optional signup token)
    func signUp(secretKey: String, homeserver: String, signupToken: String? = nil) async throws -> String {
        let result = try signUp(secretKey: secretKey, homeserver: homeserver, signupToken: signupToken)
        if result[0] == "true" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Get the homeserver for a Pubky public key
    func getHomeserver(pubky: String) async throws -> String {
        let result = try getHomeserver(pubky: pubky)
        if result[0] == "true" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Publish content
    func publishContent(recordName: String, content: String, secretKey: String) async throws -> String {
        let result = try publish(recordName: recordName, recordContent: content, secretKey: secretKey)
        if result[0] == "true" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Retrieve content
    func getContent(url: String) async throws -> String {
        let result = try get(url: url)
        if result[0] == "true" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
}
```

### Example Implementation
```swift
class ViewController: UIViewController {
    let pubkyManager = PubkyManager()
    
    func setupAccount() async {
        do {
            // Generate new account
            let secretKey = try pubkyManager.generateNewAccount()
            
            // Sign up with homeserver
            let homeserver = "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
            
            // For servers requiring signup tokens
            // let adminPassword = "your-admin-password"
            // let signupToken = try await pubkyManager.getSignupToken(homeserverPubky: homeserver, adminPassword: adminPassword)
            // let publicKey = try await pubkyManager.signUp(secretKey: secretKey, homeserver: homeserver, signupToken: signupToken)
            
            // For servers without token requirements
            let publicKey = try await pubkyManager.signUp(secretKey: secretKey, homeserver: homeserver)
            
            // Publish content
            let content = "Hello, Pubky!"
            let recordName = "example.com"
            let publishResult = try await pubkyManager.publishContent(
                recordName: recordName,
                content: content,
                secretKey: secretKey
            )
            
            print("Published with public key: \(publishResult)")
            
            // Get homeserver for a public key
            let foundHomeserver = try await pubkyManager.getHomeserver(pubky: publishResult)
            print("Homeserver for this key: \(foundHomeserver)")
        } catch {
            print("Error: \(error.localizedDescription)")
        }
    }
}
```

## Android Integration

### Installation
Add the authenticated GitHub Packages repository and a dependency version that
matches the `pubky-core-ffi` release. For local builds, set `gpr.user` to your
GitHub username and `gpr.key` to a classic personal access token with
`read:packages` in your user-level Gradle properties; do not commit them.

```kotlin
repositories {
    maven {
        url = uri("https://maven.pkg.github.com/pubky/pubky-core-ffi")
        credentials {
            username = providers.gradleProperty("gpr.user").orNull
                ?: System.getenv("GITHUB_ACTOR")
            password = providers.gradleProperty("gpr.key").orNull
                ?: System.getenv("GITHUB_TOKEN")
        }
    }
}

dependencies {
    implementation("org.pubky:pubky-core-android:0.4.0")
}
```

Initialize Android certificate verification once, before network calls:

```kotlin
uniffi.pubkycore.RustlsInit.ensure(applicationContext)
```

The AAR supports Android API 21 and newer. Consumers migrating from copied
bindings must remove their generated Kotlin, JNI libraries, and any local
`RustlsInit`/rustls verifier classes to avoid duplicate classes or libraries.

To build and verify the Maven publication locally, first run
`./build_android.sh`, then use Gradle 8.13:

```bash
gradle -p android :library:publishAllPublicationsToBuildRepository
./scripts/verify_android_maven_publication.sh android/library/build/repo 0.4.0
gradle -p android :consumer:assembleDebug :consumer:assembleDebugAndroidTest :consumer:assembleRelease
```

The independent consumer resolves the generated Maven publication, including
its transitive dependencies and consumer shrinker rules. Run
`gradle -p android :consumer:connectedDebugAndroidTest` on an emulator to verify
native loading, idempotent initialization, and an HTTPS request. For the page-size
regression test, use a 16 KB system image and confirm `adb shell getconf PAGESIZE`
returns `16384`.

### Basic Usage
```kotlin
class PubkyManager {
    init {
        RustlsInit.ensure(context)
    }
    
    fun generateNewAccount(): String {
        val result = generateSecretKey()
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        val json = JSONObject(result[1])
        return json.getString("secret_key")
    }
    
    suspend fun getSignupToken(homeserverPubky: String, adminPassword: String): String {
        val result = getSignupToken(homeserverPubky, adminPassword)
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun signUp(secretKey: String, homeserver: String, signupToken: String? = null): String {
        val result = signUp(secretKey, homeserver, signupToken)
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun getHomeserver(pubky: String): String {
        val result = getHomeserver(pubky)
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun publishContent(recordName: String, content: String, secretKey: String): String {
        val result = publish(recordName, content, secretKey)
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun getContent(url: String): String {
        val result = get(url)
        if (result[0] == "true") {
            throw Exception(result[1])
        }
        return result[1]
    }
}
```

### Example Implementation
```kotlin   
class MainActivity : AppCompatActivity() {
    private val pubkyManager by lazy { PubkyManager(applicationContext) }
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        
        lifecycleScope.launch {
            try {
                // Generate new account
                val secretKey = pubkyManager.generateNewAccount()
                
                // Sign up with homeserver
                val homeserver = "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
                
                // For servers requiring signup tokens
                // val adminPassword = "your-admin-password"
                // val signupToken = pubkyManager.getSignupToken(homeserver, adminPassword)
                // val publicKey = pubkyManager.signUp(secretKey, homeserver, signupToken)
                
                // For servers without token requirements
                val publicKey = pubkyManager.signUp(secretKey, homeserver)
                
                // Publish content
                val content = "Hello, Pubky!"
                val recordName = "example.com"
                val publishResult = pubkyManager.publishContent(
                    recordName = recordName,
                    content = content,
                    secretKey = secretKey
                )
                
                Log.d("Pubky", "Published with public key: $publishResult")
                
                // Get homeserver for a public key
                val foundHomeserver = pubkyManager.getHomeserver(publishResult)
                Log.d("Pubky", "Homeserver for this key: $foundHomeserver")
            } catch (e: Exception) {
                Log.e("Pubky", "Error: ${e.message}")
            }
        }
    }
}
```

## Advanced Features

### Working with HTTPS Records

```swift
// iOS
func publishHttps(recordName: String, target: String, secretKey: String) async throws -> String {
    let result = try publishHttps(recordName: recordName, target: target, secretKey: secretKey)
    if result[0] == "true" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

```kotlin
// Android
suspend fun publishHttps(recordName: String, target: String, secretKey: String): String {
    val result = publishHttps(recordName, target, secretKey)
    if (result[0] == "true") {
        throw Exception(result[1])
    }
    return result[1]
}
```

### User Authentication with Signup Tokens

For servers that require authentication control:

```swift
// iOS
// 1. Admin generates a signup token
func getServerSignupToken(homeserverPubky: String, adminPassword: String) async throws -> String {
    let result = try getSignupToken(homeserverPubky: homeserverPubky, adminPassword: adminPassword)
    if result[0] == "true" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}

// 2. User signs up with the token
func signUpWithToken(secretKey: String, homeserver: String, token: String) async throws -> String {
    let result = try signUp(secretKey: secretKey, homeserver: homeserver, signupToken: token)
    if result[0] == "true" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

```kotlin
// Android
// 1. Admin generates a signup token
suspend fun getServerSignupToken(homeserverPubky: String, adminPassword: String): String {
    val result = getSignupToken(homeserverPubky, adminPassword)
    if (result[0] == "true") {
        throw Exception(result[1])
    }
    return result[1]
}

// 2. User signs up with the token
suspend fun signUpWithToken(secretKey: String, homeserver: String, token: String): String {
    val result = signUp(secretKey, homeserver, token)
    if (result[0] == "true") {
        throw Exception(result[1])
    }
    return result[1]
}
```

### Getting and Using Homeserver Information

You can retrieve the homeserver information for a Pubky:

```swift
// iOS
func findHomeserver(pubkyKey: String) async throws -> String {
    let result = try getHomeserver(pubky: pubkyKey)
    if result[0] == "true" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}

// Usage example
func checkAndReconnect(pubkyKey: String) async {
    do {
        let homeserver = try await findHomeserver(pubkyKey: pubkyKey)
        print("Found homeserver: \(homeserver)")
    } catch {
        print("Failed to find homeserver: \(error.localizedDescription)")
    }
}
```

```kotlin
// Android
suspend fun findHomeserver(pubkyKey: String): String {
    val result = getHomeserver(pubkyKey)
    if (result[0] == "true") {
        throw Exception(result[1])
    }
    return result[1]
}

// Usage example
suspend fun checkAndReconnect(pubkyKey: String) {
    try {
        val homeserver = findHomeserver(pubkyKey)
        Log.d("Pubky", "Found homeserver: $homeserver")
    } catch (e: Exception) {
        Log.e("Pubky", "Failed to find homeserver: ${e.message}")
    }
}
```

### Recovery File Management
```swift
// iOS
func createRecoveryFile(secretKey: String, passphrase: String) throws -> String {
    let result = try createRecoveryFile(secretKey: secretKey, passphrase: passphrase)
    if result[0] == "true" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

```kotlin
// Android
fun createRecoveryFile(secretKey: String, passphrase: String): String {
    val result = createRecoveryFile(secretKey, passphrase)
    if (result[0] == "true") {
        throw Exception(result[1])
    }
    return result[1]
}
```

## Error Handling
All methods return a `Vec<String>` where:
- The first element ([0]) is the error flag: "true" if the call failed, "false" on success
- The second element ([1]) contains either the result data or error message

It's recommended to wrap all calls in try-catch blocks and handle errors appropriately in your application.

## Network Configuration

You can switch between default and testnet:
```swift
// iOS
try switchNetwork(useTestnet: true) // For testnet
try switchNetwork(useTestnet: false) // For default
```

```kotlin
// Android
switchNetwork(true) // For testnet
switchNetwork(false) // For default
```
