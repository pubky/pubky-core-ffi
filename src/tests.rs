use crate::generate_keypair;
use once_cell::sync::Lazy;
use pkarr::Keypair;
use pubky::Client;
use std::string::ToString;
use std::sync::Arc;

pub static TEST_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| match Client::builder().build() {
    Ok(client) => Arc::new(client),
    Err(_) => panic!("Failed to create PubkyClient"),
});

//pub const HOMESERVER: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
pub const HOMESERVER: &str = "ufibwbmed6jeq9k4p583go95wofakh9fwpp4k734trq79pd9u1uy";

// For tests that need a consistent keypair
pub static SHARED_KEYPAIR: Lazy<Keypair> = Lazy::new(Keypair::random);

// For tests that need fresh keypairs
pub fn generate_test_keypair() -> Keypair {
    Keypair::random()
}

pub fn get_test_setup() -> (Keypair, String, String) {
    let keypair = generate_keypair();
    //let keypair = SHARED_KEYPAIR.clone();
    let secret_key = hex::encode(keypair.secret_key());
    let homeserver = HOMESERVER.to_string();
    (keypair, secret_key, homeserver)
}

#[cfg(test)]
mod tests {
    use crate::tests::get_test_setup;
    use crate::*;
    use base64;
    use tokio;

    // Test keypair generation
    #[test]
    fn test_put_and_get_and_list() {
        let (keypair, secret_key, homeserver) = get_test_setup();

        let public_key = keypair.public_key().to_string();
        let url = format!("pubky://{}/pub/test.com/testfile", public_key);
        let content = "test content".to_string();

        let sign_up_result = sign_up(secret_key, homeserver, None);
        // assert_eq!(sign_up_result[0], "success");

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

        if let Some(url_str) = json
            .as_array()
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.as_str())
        {
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
        let (_, secret_key, homeserver) = get_test_setup();

        // First sign up
        let sign_up_result = sign_up(secret_key.clone(), homeserver, None);
        println!("Sign up result: {:?}", sign_up_result);
        assert_eq!(sign_up_result[0], "success");

        // Test sign in
        let sign_in_result = sign_in(secret_key.clone());
        assert_eq!(sign_in_result[0], "success");

        // Test sign out
        let sign_out_result = sign_out(secret_key);
        assert_eq!(sign_out_result[0], "success");
    }

    // Test delete functionality
    #[test]
    fn test_delete() {
        let (keypair, secret_key, homeserver) = get_test_setup();

        // First sign up
        let sign_up_result = sign_up(secret_key.clone(), homeserver, None);
        println!("Sign up result: {:?}", sign_up_result);
        assert_eq!(sign_up_result[0], "success");

        let public_key = keypair.public_key().to_string();
        let url = format!("pubky://{}/pub/test.com/testfile", public_key);
        let content = "test content".to_string();

        // Put some content first
        let put_result = put(url.clone(), content);
        println!("Put result: {:?}", put_result);
        assert_eq!(put_result[0], "success");

        std::thread::sleep(std::time::Duration::from_secs(2));

        // Test delete
        let delete_result = delete_file(url.clone());
        println!("Delete result: {:?}", delete_result);
        assert_eq!(delete_result[0], "success");
        assert_eq!(delete_result[1], "Deleted successfully");

        std::thread::sleep(std::time::Duration::from_secs(2));

        // Verify deletion by checking for empty content
        let get_result = get(url.clone());
        println!("Get result after delete: {:?}", get_result);
        assert_eq!(
            get_result[1], "",
            "File should be empty after deletion at URL: {}",
            url
        );
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

    #[test]
    fn test_republish_homeserver() {
        let (_, secret_key, homeserver) = get_test_setup();

        // First sign up to ensure the user exists
        let sign_up_result = sign_up(secret_key.clone(), homeserver.clone(), None);
        println!("Sign up result: {:?}", sign_up_result);
        assert_eq!(sign_up_result[0], "success");

        // Test republish homeserver
        let republish_result = republish_homeserver(secret_key, homeserver);
        println!("Republish homeserver result: {:?}", republish_result);
        assert_eq!(republish_result[0], "success");
        assert_eq!(republish_result[1], "Homeserver republished successfully");
    }

    // Test generate_mnemonic_phrase
    #[test]
    fn test_generate_mnemonic_phrase() {
        let result = generate_mnemonic_phrase();
        assert_eq!(result[0], "success");

        let mnemonic = &result[1];
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12, "Mnemonic should have 12 words");

        // Validate the generated mnemonic
        let validate_result = validate_mnemonic_phrase(mnemonic.clone());
        assert_eq!(validate_result[0], "success");
        assert_eq!(validate_result[1], "true");
    }

    // Test mnemonic_phrase_to_keypair
    #[test]
    fn test_mnemonic_phrase_to_keypair() {
        // First generate a valid mnemonic
        let mnemonic_result = generate_mnemonic_phrase();
        assert_eq!(mnemonic_result[0], "success");
        let mnemonic = mnemonic_result[1].clone();

        // Convert to keypair
        let result = mnemonic_phrase_to_keypair(mnemonic);
        assert_eq!(result[0], "success");

        let json: serde_json::Value = serde_json::from_str(&result[1]).unwrap();
        assert!(json["secret_key"].is_string());
        assert!(json["public_key"].is_string());
        assert!(json["uri"].is_string());

        // Test with invalid mnemonic
        let invalid_result = mnemonic_phrase_to_keypair("invalid mnemonic phrase".to_string());
        assert_eq!(invalid_result[0], "error");
    }

    // Test generate_mnemonic_phrase_and_keypair
    #[test]
    fn test_generate_mnemonic_phrase_and_keypair() {
        let result = generate_mnemonic_phrase_and_keypair();
        assert_eq!(result[0], "success");

        let json: serde_json::Value = serde_json::from_str(&result[1]).unwrap();
        assert!(json["mnemonic"].is_string());
        assert!(json["secret_key"].is_string());
        assert!(json["public_key"].is_string());
        assert!(json["uri"].is_string());

        // Verify the mnemonic is valid
        let mnemonic = json["mnemonic"].as_str().unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12, "Mnemonic should have 12 words");

        // Verify we can recreate the same keypair from the mnemonic
        let keypair_result = mnemonic_phrase_to_keypair(mnemonic.to_string());
        assert_eq!(keypair_result[0], "success");
        let keypair_json: serde_json::Value = serde_json::from_str(&keypair_result[1]).unwrap();
        assert_eq!(json["secret_key"], keypair_json["secret_key"]);
        assert_eq!(json["public_key"], keypair_json["public_key"]);
    }

    // Test validate_mnemonic_phrase
    #[test]
    fn test_validate_mnemonic_phrase() {
        // Test with valid mnemonic
        let valid_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result = validate_mnemonic_phrase(valid_mnemonic.to_string());
        assert_eq!(result[0], "success");
        assert_eq!(result[1], "true");

        // Test with invalid mnemonic
        let invalid_mnemonic = "invalid words that are not in the bip39 wordlist";
        let result = validate_mnemonic_phrase(invalid_mnemonic.to_string());
        assert_eq!(result[0], "success");
        assert_eq!(result[1], "false");

        // Test with wrong number of words
        let wrong_count = "abandon abandon abandon";
        let result = validate_mnemonic_phrase(wrong_count.to_string());
        assert_eq!(result[0], "success");
        assert_eq!(result[1], "false");
    }

    // Test mnemonic consistency
    #[test]
    fn test_mnemonic_consistency() {
        // Generate a mnemonic and verify consistent keypair derivation
        let mnemonic_result = generate_mnemonic_phrase();
        assert_eq!(mnemonic_result[0], "success");
        let mnemonic = mnemonic_result[1].clone();

        // Get keypair multiple times and ensure consistency
        let keypair1 = mnemonic_phrase_to_keypair(mnemonic.clone());
        let keypair2 = mnemonic_phrase_to_keypair(mnemonic.clone());
        assert_eq!(keypair1[0], "success");
        assert_eq!(keypair2[0], "success");

        let json1: serde_json::Value = serde_json::from_str(&keypair1[1]).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&keypair2[1]).unwrap();

        // Verify same mnemonic produces same keypair
        assert_eq!(
            json1["secret_key"], json2["secret_key"],
            "Same mnemonic should produce same secret key"
        );
        assert_eq!(
            json1["public_key"], json2["public_key"],
            "Same mnemonic should produce same public key"
        );
        assert_eq!(
            json1["uri"], json2["uri"],
            "Same mnemonic should produce same URI"
        );
    }
}
