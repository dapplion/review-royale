//! Webhook signature verification

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verify the webhook signature from GitHub
///
/// `signature` is the value of the `X-Hub-Signature-256` header
/// `secret` is your webhook secret
/// `body` is the raw request body
pub fn verify_signature(signature: &str, secret: &str, body: &[u8]) -> bool {
    // Signature format: "sha256=<hex digest>"
    let signature = match signature.strip_prefix("sha256=") {
        Some(s) => s,
        None => return false,
    };

    let signature_bytes = match hex::decode(signature) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    mac.update(body);
    mac.verify_slice(&signature_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature() {
        let secret = "test-secret";
        let body = b"test body";

        // Generate a valid signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let result = mac.finalize();
        let signature = format!("sha256={}", hex::encode(result.into_bytes()));

        assert!(verify_signature(&signature, secret, body));
    }

    #[test]
    fn test_invalid_signature() {
        let secret = "test-secret";
        let body = b"test body";
        let signature = "sha256=invalid";

        assert!(!verify_signature(signature, secret, body));
    }
}
