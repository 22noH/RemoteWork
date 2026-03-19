use argon2::{Argon2, PasswordHash, PasswordVerifier};
use rand::Rng;

/// Verify a plaintext password against an Argon2id PHC hash stored by the host.
/// The viewer sends the plaintext password over WSS; the server verifies it
/// against the Argon2id hash the host registered with.
pub fn verify_password(submitted_password: &str, stored_hash: &str) -> bool {
    PasswordHash::new(stored_hash)
        .map(|h| Argon2::default().verify_password(submitted_password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Generate a random 9-digit host ID
pub fn generate_host_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{:09}", rng.gen_range(100_000_000u32..999_999_999u32))
}

/// Generate a random 6-character alphanumeric password (unambiguous chars)
pub fn generate_password() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..6).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_host_id() {
        let id = generate_host_id();
        assert_eq!(id.len(), 9);
        assert!(id.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_password() {
        let pw = generate_password();
        assert_eq!(pw.len(), 6);
    }
}
