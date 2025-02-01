use sha2::{Digest, Sha256};

// encoding the long url 
pub async fn encode_long_url(url: &String) -> String {
    let hash = Sha256::digest(url.as_bytes());
    bs58::encode(hash).into_string()
}

// checking validity of the long url
pub fn valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}
