use bip39::{Language, Mnemonic};
use pkarr::Keypair;

/**
 * Get a keypair from a secret key
 */
pub fn get_keypair_from_secret_key(secret_key: &str) -> Result<Keypair, String> {
    let bytes = match hex::decode(&secret_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to decode secret key".to_string()),
    };

    let secret_key_bytes: [u8; 32] = match bytes.try_into() {
        Ok(secret_key) => secret_key,
        Err(_) => {
            return Err("Failed to convert secret key to 32-byte array".to_string());
        }
    };

    Ok(Keypair::from_secret_key(&secret_key_bytes))
}

/**
 * Get the secret key from a keypair
 */
pub fn get_secret_key_from_keypair(keypair: &Keypair) -> String {
    hex::encode(keypair.secret_key())
}

/**
 * Generate a new keypair
 */
pub fn generate_keypair() -> Keypair {
    Keypair::random()
}

/**
 * Generate a 12-word mnemonic phrase
 */
pub fn generate_mnemonic() -> Result<String, String> {
    // Generate a 128-bit entropy for 12 words
    let mnemonic = Mnemonic::generate_in(Language::English, 12)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}

/**
 * Convert a mnemonic phrase to a secret key
 */
pub fn mnemonic_to_secret_key(mnemonic_phrase: &str) -> Result<String, String> {
    // Parse and validate the mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic_phrase)
        .map_err(|_| "Invalid mnemonic phrase".to_string())?;

    // Convert to seed (using empty passphrase)
    let seed = mnemonic.to_seed("");

    // Take first 32 bytes as the ed25519 secret key
    let secret_key_bytes: [u8; 32] = seed[..32]
        .try_into()
        .map_err(|_| "Failed to extract secret key from seed".to_string())?;

    // Convert to hex string
    Ok(hex::encode(secret_key_bytes))
}

/**
 * Convert a mnemonic phrase to a keypair
 */
pub fn mnemonic_to_keypair(mnemonic_phrase: &str) -> Result<Keypair, String> {
    let secret_key = mnemonic_to_secret_key(mnemonic_phrase)?;
    get_keypair_from_secret_key(&secret_key)
}

/**
 * Generate a new mnemonic and return both the mnemonic and the derived keypair
 */
pub fn generate_mnemonic_and_keypair() -> Result<(String, Keypair), String> {
    let mnemonic = generate_mnemonic()?;
    let keypair = mnemonic_to_keypair(&mnemonic)?;
    Ok((mnemonic, keypair))
}

/**
 * Validate a mnemonic phrase
 */
pub fn validate_mnemonic(mnemonic_phrase: &str) -> bool {
    Mnemonic::parse_in(Language::English, mnemonic_phrase).is_ok()
}
