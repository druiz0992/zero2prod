use actix_web::HttpRequest;
use anyhow::Context;

use crate::domain::auth::credentials::Credentials;

pub fn basic_authentication(request: HttpRequest) -> Result<Credentials, anyhow::Error> {
    let headers = request.headers();
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF8 String.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was no 'Basic'")?;
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decided credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials::new(username, password))
}
