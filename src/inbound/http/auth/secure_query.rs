use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

#[derive(serde::Deserialize)]
pub struct SecureQuery {
    pub error: String,
    pub tag: String,
}

impl SecureQuery {
    pub fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::encode(&self.error));
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        let decoded_error =
            urlencoding::decode(&self.error.strip_prefix("error=").unwrap_or(&self.error))?
                .into_owned();
        Ok(decoded_error)
    }
    pub fn new(error: String, secret: &HmacSecret) -> Self {
        let query_string = format!("error={}", urlencoding::encode(&error));
        let hmac_tag = {
            let mut mac =
                Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };
        Self {
            error: query_string,
            tag: hex::encode(&hmac_tag),
        }
    }

    pub fn query(&self) -> &str {
        &self.error
    }
    pub fn tag(&self) -> &str {
        &self.tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use secrecy::Secret;
    use serde_json;
    use sha2::Sha256;

    // Helper function to create a valid HmacSecret for testing
    fn create_hmac_secret() -> HmacSecret {
        let secret = Secret::new("my_secret_key".to_string());
        HmacSecret(secret)
    }

    #[test]
    fn test_deserialization() {
        let json_data = r#"{"error":"error=Authentication error: Invalid password","tag":"3d1cda86e7a1e639aa82edce86aa20f43f3e7d4f09f826a237ce1da7c18d9e2b"}"#;

        let secure_query: SecureQuery = serde_json::from_str(json_data).unwrap();

        assert_eq!(
            secure_query.error,
            "error=Authentication error: Invalid password"
        );
        assert_eq!(
            secure_query.tag,
            "3d1cda86e7a1e639aa82edce86aa20f43f3e7d4f09f826a237ce1da7c18d9e2b"
        );
    }

    #[test]
    fn test_verify_success() {
        let secret = create_hmac_secret();
        //let error_message = "error=Authentication error: Invalid password".to_string();
        let error_message = "error".to_string();

        // Create a SecureQuery instance
        let secure_query = SecureQuery::new(error_message.clone(), &secret);

        // Verify the query
        let result = secure_query.verify(&secret);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Authentication error: Invalid password");
    }

    #[test]
    fn test_verify_invalid_tag() {
        let secret = create_hmac_secret();
        let error_message = "error=Authentication error: Invalid password".to_string();

        // Create a SecureQuery instance
        let mut secure_query = SecureQuery::new(error_message.clone(), &secret);

        // Modify the tag to an invalid one
        secure_query.tag = "invalid_tag".to_string();

        // Verify the query
        let result = secure_query.verify(&secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_invalid_decoding() {
        let secret = create_hmac_secret();

        // Create a SecureQuery instance with a malformed tag
        let secure_query = SecureQuery {
            error: "error=Malformed query".to_string(),
            tag: "invalid_hex".to_string(),
        };

        // Verify the query
        let result = secure_query.verify(&secret);

        assert!(result.is_err());
    }
}
