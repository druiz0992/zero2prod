use crate::{
    domain::newsletter::errors::NewsletterError, outbound::telemetry::spawn_blocking_with_tracing,
};
use anyhow::{Context, Result};
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize, Clone)]
pub struct Credentials {
    username: String,
    password: Secret<String>,
}

impl Credentials {
    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn new(username: String, password: String) -> Self {
        Self {
            username,
            password: Secret::new(password),
        }
    }

    pub fn password(&self) -> Secret<String> {
        self.password.clone()
    }

    #[tracing::instrument(name = "Validate credentials", skip(self, stored_credentials))]
    pub async fn validate(
        self,
        stored_credentials: Option<StoredCredentials>,
    ) -> Result<uuid::Uuid, CredentialsError> {
        let stored_credentials = stored_credentials.unwrap_or_default();
        let user_id = stored_credentials.user_id;

        spawn_blocking_with_tracing(move || stored_credentials.verify(self.password))
            .await
            .context("Failed to spawn a blocking task.")
            .map_err(CredentialsError::Unexpected)??;

        if user_id.is_nil() {
            return Err(CredentialsError::AuthError("Unknown username.".to_string()));
        }

        Ok(user_id)
    }
}

pub struct StoredCredentials {
    user_id: uuid::Uuid,
    password_hash: Secret<String>,
}

static DEFAULT_PASSWORD_HASH: Lazy<Secret<String>> = Lazy::new(|| {
    Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    )
});

impl StoredCredentials {
    pub fn new(user_id: uuid::Uuid, password_hash: String) -> Self {
        Self {
            user_id,
            password_hash: Secret::new(password_hash),
        }
    }

    fn verify(&self, password_candidate: Secret<String>) -> Result<(), CredentialsError> {
        let expected_password_hash = PasswordHash::new(self.password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")
            .map_err(CredentialsError::Unexpected)?;

        Argon2::default()
            .verify_password(
                password_candidate.expose_secret().as_bytes(),
                &expected_password_hash,
            )
            .map_err(|_| CredentialsError::AuthError("Invalid password.".to_string()))
    }
    pub fn user_id(&self) -> uuid::Uuid {
        self.user_id
    }
    pub fn password_hash(&self) -> &str {
        self.password_hash.expose_secret()
    }
}

impl Default for StoredCredentials {
    fn default() -> Self {
        Self {
            user_id: uuid::Uuid::nil(),
            password_hash: DEFAULT_PASSWORD_HASH.clone(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CredentialsError {
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<CredentialsError> for NewsletterError {
    fn from(error: CredentialsError) -> Self {
        match error {
            CredentialsError::AuthError(e) => NewsletterError::AuthError(e),
            CredentialsError::Unexpected(e) => NewsletterError::Unexpected(e),
        }
    }
}

pub fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}
