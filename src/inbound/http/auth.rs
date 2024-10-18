use actix_web::HttpRequest;

use crate::domain::auth::credentials::{Credentials, CredentialsError};

pub fn basic_authentication(request: HttpRequest) -> Result<Credentials, CredentialsError> {
    let headers = request.headers();
    let header_value = headers
        .get("Authorization")
        .ok_or(CredentialsError::AuthError(format!(
            "The 'Authorization' header is missing"
        )))?
        .to_str()
        .map_err(|e| CredentialsError::AuthError(e.to_string()))?;
    let base64encoded_segment =
        header_value
            .strip_prefix("Basic ")
            .ok_or(CredentialsError::AuthError(format!(
                "The authorization scheme was no 'Basic'"
            )))?;
    let decoded_bytes =
        base64::decode_config(base64encoded_segment, base64::STANDARD).map_err(|_| {
            CredentialsError::AuthError(format!("Failed to base64-decode 'Basic' credentials"))
        })?;
    let decoded_credentials = String::from_utf8(decoded_bytes).map_err(|_| {
        CredentialsError::AuthError(format!("The decided credential string is not valid UTF8."))
    })?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| {
            CredentialsError::AuthError(format!("A username must be provided in 'Basic' auth."))
        })?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| {
            CredentialsError::AuthError(format!("A password must be provided in 'Basic' auth."))
        })?
        .to_string();

    Ok(Credentials::new(username, password))
}
