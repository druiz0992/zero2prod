use crate::{
    domain::newsletter::errors::NewsletterError, outbound::telemetry::spawn_blocking_with_tracing,
};
use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use once_cell::sync::Lazy;
use secrecy::{ExposeSecret, Secret};

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

    #[tracing::instrument(name = "Validate credentials", skip(self, stored_credentials))]
    pub async fn validate_credentials(
        self,
        stored_credentials: Option<StoredCredentials>,
    ) -> Result<uuid::Uuid, CredentialsError> {
        let stored_credentials = match stored_credentials {
            Some(credentials) => credentials,
            None => StoredCredentials::default(),
        };
        let user_id = stored_credentials.user_id;

        spawn_blocking_with_tracing(move || stored_credentials.verify(self.password))
            .await
            .context("Failed to spawn a blocking task.")
            .map_err(CredentialsError::Unexpected)??;

        if user_id.is_nil() {
            return Err(CredentialsError::AuthError(format!("Unknown username.")));
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
            .map_err(|e| CredentialsError::Unexpected(e))?;

        Argon2::default()
            .verify_password(
                password_candidate.expose_secret().as_bytes(),
                &expected_password_hash,
            )
            .map_err(|_| CredentialsError::AuthError(format!("Invalid password")))
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
