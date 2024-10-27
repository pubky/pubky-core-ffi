# Pubky Core Mobile SDK

The Pubky Core Mobile SDK provides native bindings for iOS and Android platforms to interact with Pubky. This SDK allows you to perform operations like publishing content, retrieving data and managing authentication.

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

## Run Tests:
```
cargo test -- --test-threads=1
```

## iOS Integration

### Installation
1. Add the XCFramework to your Xcode project:

   - Drag bindings/ios/PubkyMobile.xcframework into your Xcode project
Ensure "Copy items if needed" is checked
Add the framework to your target


2. Copy the Swift bindings:

   - Add bindings/ios/pubkymobile.swift to your project

### Basic Usage
```swift
import PubkyMobile
import PubkyMobile

class PubkyManager {
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
    
    // Sign up with a homeserver
    func signUp(secretKey: String, homeserver: String) async throws -> String {
        let result = try signUp(secretKey: secretKey, homeserver: homeserver)
        if result[0] == "error" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Publish content
    func publishContent(recordName: String, content: String, secretKey: String) async throws -> String {
        let result = try publish(recordName: recordName, recordContent: content, secretKey: secretKey)
        if result[0] == "error" {
            throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
        }
        return result[1]
    }
    
    // Retrieve content
    func getContent(url: String) async throws -> String {
        let result = try get(url: url)
        if result[0] == "error" {
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
        } catch {
            print("Error: \(error.localizedDescription)")
        }
    }
}
```

## Android Integration

### Installation
1. Add the JNI libraries to your project:

   - Copy the contents of bindings/android/jniLibs to your project's app/src/main/jniLibs directory


2. Add the Kotlin bindings:

   - Copy bindings/android/pubkymobile.kt to your project's source directory

### Basic Usage
```kotlin
class PubkyManager {
    init {
        // Initialize the library
        System.loadLibrary("pubkymobile")
    }
    
    fun generateNewAccount(): String {
        val result = generateSecretKey()
        if (result[0] == "error") {
            throw Exception(result[1])
        }
        val json = JSONObject(result[1])
        return json.getString("secret_key")
    }
    
    suspend fun signUp(secretKey: String, homeserver: String): String {
        val result = signUp(secretKey, homeserver)
        if (result[0] == "error") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun publishContent(recordName: String, content: String, secretKey: String): String {
        val result = publish(recordName, content, secretKey)
        if (result[0] == "error") {
            throw Exception(result[1])
        }
        return result[1]
    }
    
    suspend fun getContent(url: String): String {
        val result = get(url)
        if (result[0] == "error") {
            throw Exception(result[1])
        }
        return result[1]
    }
}
```

### Example Implementation
```kotlin   
class MainActivity : AppCompatActivity() {
    private val pubkyManager = PubkyManager()
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        
        lifecycleScope.launch {
            try {
                // Generate new account
                val secretKey = pubkyManager.generateNewAccount()
                
                // Sign up with homeserver
                val homeserver = "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
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
    if result[0] == "error" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

```kotlin
// Android
suspend fun publishHttps(recordName: String, target: String, secretKey: String): String {
    val result = publishHttps(recordName, target, secretKey)
    if (result[0] == "error") {
        throw Exception(result[1])
    }
    return result[1]
}
```

### Recovery File Management
```swift
// iOS
func createRecoveryFile(secretKey: String, passphrase: String) throws -> String {
    let result = try createRecoveryFile(secretKey: secretKey, passphrase: passphrase)
    if result[0] == "error" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

```kotlin
// iOS
func createRecoveryFile(secretKey: String, passphrase: String) throws -> String {
    let result = try createRecoveryFile(secretKey: secretKey, passphrase: passphrase)
    if result[0] == "error" {
        throw NSError(domain: "PubkyError", code: -1, userInfo: [NSLocalizedDescriptionKey: result[1]])
    }
    return result[1]
}
```

## Error Handling
All methods return a `Vec<String>` where:
   - The first element ([0]) is either "success" or "error"
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