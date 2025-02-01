use sha2::{Digest, Sha256};

// encoding the long url 
pub async fn encode_long_url(url: &String) -> String {
    let hash = Sha256::digest(url.as_bytes());
    bs58::encode(hash).into_string()
}

// validation for long url
pub fn valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

// short code validation
pub fn valid_short_code(short_code: &str) -> bool {
    if short_code.len() != 8 {
        return false;
    }
    bs58::decode(short_code).into_vec().is_ok()
}