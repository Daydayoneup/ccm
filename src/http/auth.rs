use sha2::{Sha256, Digest};

/// Generate a cryptographically random 64-char hex token (two UUIDs concatenated).
pub fn generate_token() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{}{}", a, b)
}

/// SHA-256 hash a token string, returning hex digest.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Constant-time token verification: hash the candidate and compare to stored hash.
pub fn verify_token(candidate: &str, stored_hash: &str) -> bool {
    let candidate_hash = hash_token(candidate);
    if candidate_hash.len() != stored_hash.len() {
        return false;
    }
    let mut result = 0u8;
    for (a, b) in candidate_hash.bytes().zip(stored_hash.bytes()) {
        result |= a ^ b;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_is_64_hex_chars() {
        let token = generate_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token_is_unique() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_hash_token_deterministic() {
        let token = "abc123";
        let h1 = hash_token(token);
        let h2 = hash_token(token);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_verify_token_success() {
        let token = generate_token();
        let hash = hash_token(&token);
        assert!(verify_token(&token, &hash));
    }

    #[test]
    fn test_verify_token_failure() {
        let token = generate_token();
        let hash = hash_token("wrong-token");
        assert!(!verify_token(&token, &hash));
    }
}
