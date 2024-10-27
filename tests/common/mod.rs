use std::string::ToString;
use std::sync::Arc;
use once_cell::sync::Lazy;
use pubky::PubkyClient;
use pkarr::Keypair;

pub static TEST_CLIENT: Lazy<Arc<PubkyClient>> = Lazy::new(|| {
    Arc::new(PubkyClient::default())
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
    let keypair = SHARED_KEYPAIR.clone();
    let secret_key = hex::encode(keypair.secret_key());
    let homeserver = HOMESERVER.to_string();
    (keypair, secret_key, homeserver)
}