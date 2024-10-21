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
        let decoded_error = urlencoding::decode(&self.error)?.into_owned();
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
