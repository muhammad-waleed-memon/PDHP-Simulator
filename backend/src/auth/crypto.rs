//! Password hashing and verification utilities using Argon2id.

use crate::error::{AppError, AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hashes a plaintext password using Argon2id with a cryptographically random salt.
/// Returns the PHC-formatted password hash string.
/// Never logs the input password or generated hash.
pub fn hash_password(password: &str) -> AppResult<String> {
    if password.trim().is_empty() {
        return Err(AppError::BadRequest("Password cannot be empty".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::internal(format!("Password security operation failed: {e}")))
}

/// Verifies a plaintext password against an Argon2id PHC password hash string.
/// Returns true if password matches, false otherwise.
/// Never logs the input password or hash.
pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| AppError::InvalidCredentials)?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "SecurePassword123!";
        let hash = hash_password(password).expect("Hashing should succeed");

        assert_ne!(password, hash);
        assert!(hash.starts_with("$argon2"));

        // Correct password verifies
        assert!(verify_password(password, &hash).expect("Verification should complete"));

        // Incorrect password fails
        assert!(!verify_password("WrongPassword123!", &hash).expect("Verification should complete"));
    }

    #[test]
    fn test_empty_password_rejected() {
        assert!(hash_password("").is_err());
        assert!(hash_password("   ").is_err());
    }
}
