pub mod commands;
pub mod config;
pub mod crypto;
pub mod gitignore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let identity = crypto::generate_identity();
        let recipient = crypto::identity_to_recipient(&identity);
        let plaintext = b"MY_SECRET=super_secret_value\nANOTHER=12345";

        let ciphertext = crypto::encrypt_bytes(plaintext, &[recipient]).unwrap();
        let decrypted = crypto::decrypt_bytes(&ciphertext, &identity).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let identity1 = crypto::generate_identity();
        let recipient1 = crypto::identity_to_recipient(&identity1);

        let identity2 = crypto::generate_identity();

        let plaintext = b"SHOULD_NOT_SEE_THIS";
        let ciphertext = crypto::encrypt_bytes(plaintext, &[recipient1]).unwrap();

        let result = crypto::decrypt_bytes(&ciphertext, &identity2);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Decryption failed");
    }

    #[test]
    fn identity_to_string_and_back() {
        let identity = crypto::generate_identity();
        let s = crypto::identity_to_string(&identity);
        assert!(s.starts_with("AGE-SECRET-KEY-"));

        let parsed = crypto::parse_identity(&s).unwrap();
        let orig_recipient = crypto::identity_to_recipient(&identity).to_string();
        let parsed_recipient = crypto::identity_to_recipient(&parsed).to_string();
        assert_eq!(orig_recipient, parsed_recipient);
    }

    #[test]
    fn parse_invalid_identity_fails() {
        let result = crypto::parse_identity("not-a-valid-key");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_recipient_fails() {
        let result = crypto::parse_recipient("not-a-valid-recipient");
        assert!(result.is_err());
    }

    #[test]
    fn recipient_derived_from_identity() {
        let identity = crypto::generate_identity();
        let recipient = crypto::identity_to_recipient(&identity);
        let recipient_str = recipient.to_string();
        assert!(recipient_str.starts_with("age1"));

        let parsed = crypto::parse_recipient(&recipient_str).unwrap();
        assert_eq!(parsed.to_string(), recipient_str);
    }

    #[test]
    fn multi_recipient_encrypt_decrypt() {
        let id1 = crypto::generate_identity();
        let id2 = crypto::generate_identity();
        let rec1 = crypto::identity_to_recipient(&id1);
        let rec2 = crypto::identity_to_recipient(&id2);

        let plaintext = b"SHARED_SECRET=both_can_read";

        let ciphertext = crypto::encrypt_bytes(plaintext, &[rec1, rec2]).unwrap();

        let dec1 = crypto::decrypt_bytes(&ciphertext, &id1).unwrap();
        let dec2 = crypto::decrypt_bytes(&ciphertext, &id2).unwrap();
        assert_eq!(dec1, plaintext);
        assert_eq!(dec2, plaintext);
    }
}
