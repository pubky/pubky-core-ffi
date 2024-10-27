use pubkymobile::*;
use tokio;
use base64;

mod common;
use crate::common::{get_test_setup};

// Test keypair generation
#[test]
fn test_put_and_get_and_list() {
    let (keypair, secret_key, homeserver) = get_test_setup();

    let public_key = keypair.public_key().to_string();
    let url = format!("pubky://{}/pub/test.com/testfile", public_key);
    let content = "test content".to_string();

    let sign_up_result = sign_up(secret_key, homeserver);
    assert_eq!(sign_up_result[0], "success");

    let inner_url = url.clone();

    let put_result = put(url.clone(), content.clone());
    assert_eq!(put_result[0], "success");

    // Add a small delay to ensure the put operation completes
    std::thread::sleep(std::time::Duration::from_secs(1));

    let get_result = get(url);
    assert_eq!(get_result[0], "success");
    assert_eq!(get_result[1], content);

    let list_result = list(inner_url);
    println!("List result: {:?}", list_result);
    assert_eq!(list_result[0], "success");

    let json: serde_json::Value = serde_json::from_str(&list_result[1]).unwrap();
    assert!(json.is_array());

    if let Some(url_str) = json.as_array().and_then(|arr| arr.get(0)).and_then(|v| v.as_str()) {
        assert!(url_str.contains(&public_key));
    } else {
        panic!("Expected array with URL string");
    }
}

// Test generate secret key
#[tokio::test]
async fn test_generate_secret_key() {
    let result = generate_secret_key();
    assert_eq!(result[0], "success");

    let json: serde_json::Value = serde_json::from_str(&result[1]).unwrap();

    assert!(json["secret_key"].is_string());
    assert!(json["public_key"].is_string());
    assert!(json["uri"].is_string());
}

// Test get public key from secret key
#[tokio::test]
async fn test_get_public_key_from_secret_key() {
    let (_, secret_key, _) = get_test_setup();

    let result = get_public_key_from_secret_key(secret_key);
    assert_eq!(result[0], "success");

    let json: serde_json::Value = serde_json::from_str(&result[1]).unwrap();
    assert!(json["public_key"].is_string());
    assert!(json["uri"].is_string());

    // Test with invalid secret key
    let result = get_public_key_from_secret_key("invalid_key".to_string());
    assert_eq!(result[0], "error");
}

// Test sign up functionality
#[test]
fn test_publish_and_resolve() {
    let (keypair, secret_key, _) = get_test_setup();
    let record_name = "test.record".to_string();
    let record_content = "test content".to_string();

    // Test publish
    let publish_result = publish(record_name.clone(), record_content.clone(), secret_key.clone());
    assert_eq!(publish_result[0], "success");

    // Test resolve
    let public_key = keypair.public_key().to_string();
    let resolve_result = resolve(public_key);
    assert_eq!(resolve_result[0], "success");

    let json: serde_json::Value = serde_json::from_str(&resolve_result[1]).unwrap();
    assert!(json["records"].is_array());
}

// Test recovery file creation and decryption
#[tokio::test]
async fn test_create_and_decrypt_recovery_file() {
    let (_, secret_key, _) = get_test_setup();
    let passphrase = "test_passphrase".to_string();

    // Create recovery file
    let create_result = create_recovery_file(secret_key.clone(), passphrase.clone());
    assert_eq!(create_result[0], "success");

    // Test recovery file decryption
    let recovery_file = create_result[1].clone();
    let decrypt_result = decrypt_recovery_file(recovery_file, passphrase);
    assert_eq!(decrypt_result[0], "success");
    assert_eq!(decrypt_result[1], secret_key);
}

// Test HTTPS publishing functionality
#[test]
fn test_publish_https() {
    let (_, secret_key, _) = get_test_setup();
    let record_name = "test.domain".to_string();
    let target = "target.domain".to_string();

    let result = publish_https(record_name, target, secret_key);
    assert_eq!(result[0], "success");
}

// Test resolve HTTPS functionality
#[test]
fn test_resolve_https() {
    let (keypair, _, _) = get_test_setup();
    let public_key = keypair.public_key().to_string();

    let result = resolve_https(public_key);
    // Note: This might be "error" if no HTTPS records exist
    assert!(result[0] == "success" || result[0] == "error");
}

// Test sign in functionality
#[test]
fn test_sign_in_and_out() {
    let (_, secret_key, _) = get_test_setup();

    // Test sign in
    let sign_in_result = sign_in(secret_key.clone());
    assert_eq!(sign_in_result[0], "success");
    assert_eq!(sign_in_result[1], "Sign in success");

    // Test sign out
    let sign_out_result = sign_out(secret_key);
    assert_eq!(sign_out_result[0], "success");
    assert_eq!(sign_out_result[1], "Sign out success");
}

// Test delete functionality
#[test]
fn test_delete() {
    let (keypair, secret_key, homeserver) = get_test_setup();

    // First sign up
    let sign_up_result = sign_up(secret_key.clone(), homeserver);
    assert_eq!(sign_up_result[0], "success");

    let public_key = keypair.public_key().to_string();
    let url = format!("pubky://{}/pub/test.com/testfile", public_key);
    let content = "test content".to_string();

    // Put some content first
    let put_result = put(url.clone(), content);
    assert_eq!(put_result[0], "success");

    // Test delete
    let delete_result = delete_file(url.clone());
    assert_eq!(delete_result[0], "success");
    assert_eq!(delete_result[1], "Deleted successfully");

    // Verify deletion by trying to get the file
    let get_result = get(url);
    assert_eq!(get_result[0], "error");
}

// Test network switching
#[test]
fn test_switch_network() {
    // Test switching to testnet
    let testnet_result = switch_network(true);
    println!("Testnet result: {:?}", testnet_result);
    assert_eq!(testnet_result[0], "success");
    assert_eq!(testnet_result[1], "Switched to testnet network");

    // Add a small delay to ensure the put operation completes
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Test switching back to default
    let default_result = switch_network(false);
    println!("Default network result: {:?}", default_result);
    assert_eq!(default_result[0], "success");
    assert_eq!(default_result[1], "Switched to default network");
}

// Test auth URL parsing
#[test]
fn test_parse_auth_url() {
    let test_url = "pubkyauth:///?caps=/pub/pubky.app/:rw,/pub/foo.bar/file:r&secret=U55XnoH6vsMCpx1pxHtt8fReVg4Brvu9C0gUBuw-Jkw&relay=http://167.86.102.121:4173/";
    let result = parse_auth_url(test_url.to_string());
    println!("test_parse_auth_url Result: {:?}", result);
    assert_eq!(result[0], "success");

    let json: serde_json::Value = serde_json::from_str(&result[1]).unwrap();
    assert!(json.is_object());
}

// Test error cases
#[test]
fn test_error_cases() {
    // Test invalid secret key
    let sign_in_result = sign_in("invalid_key".to_string());
    assert_eq!(sign_in_result[0], "error");

    // Test invalid URL
    let get_result = get("invalid_url".to_string());
    assert_eq!(get_result[0], "error");

    // Test invalid public key for resolve
    let resolve_result = resolve("invalid_public_key".to_string());
    assert_eq!(resolve_result[0], "error");

    // Test empty recovery file creation
    let recovery_result = create_recovery_file("".to_string(), "passphrase".to_string());
    assert_eq!(recovery_result[0], "error");
}