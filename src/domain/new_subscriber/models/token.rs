use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionTokenError {
    #[error(
        "Subscription token length is too long (maximum allowed is {} characters)",
        SubscriptionToken::MAX_LENGTH
    )]
    TooLong,
    #[error(
        "Subscription token length is too short (minimum allowed is {} characters)",
        SubscriptionToken::MIN_LENGTH
    )]
    TooShort,
    #[error("Subscriber name contains forbidden characters: {0}")]
    ContainsForbiddenCharacters(String),
}

#[derive(serde::Deserialize, Debug)]
pub struct SubscriptionTokenRequest {
    pub subscription_token: String,
}

impl TryFrom<SubscriptionTokenRequest> for SubscriptionToken {
    type Error = SubscriptionTokenError;
    fn try_from(value: SubscriptionTokenRequest) -> Result<Self, Self::Error> {
        SubscriptionToken::parse(value.subscription_token)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    const MAX_LENGTH: usize = 40;
    const MIN_LENGTH: usize = 20;
    const DEFAULT_LENGTH: usize = 25;

    pub fn parse(s: String) -> Result<SubscriptionToken, SubscriptionTokenError> {
        if s.len() > Self::MAX_LENGTH {
            return Err(SubscriptionTokenError::TooLong);
        }
        if s.len() < Self::MIN_LENGTH {
            return Err(SubscriptionTokenError::TooShort);
        }
        if !is_alphanumeric(&s) {
            return Err(SubscriptionTokenError::ContainsForbiddenCharacters(
                s.clone(),
            ));
        }
        Ok(Self(s))
    }

    pub fn new() -> Self {
        let mut rng = thread_rng();
        let token = std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(SubscriptionToken::DEFAULT_LENGTH)
            .collect();
        SubscriptionToken::parse(token).unwrap()
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubscriptionToken {
    type Error = SubscriptionTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SubscriptionToken::parse(value)
    }
}

impl Default for SubscriptionToken {
    fn default() -> Self {
        Self::new()
    }
}

fn is_alphanumeric(token: &str) -> bool {
    token.chars().all(|c| c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::SubscriptionToken;
    use claim::{assert_err, assert_ok};
    use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};

    #[test]
    fn empty_token_is_invalid() {
        assert_err!(SubscriptionToken::parse("".to_string()));
    }

    #[test]
    fn short_token_is_invalid() {
        let token = "a".repeat(19);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn long_token_is_invalid() {
        let token = "a".repeat(41);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn tokens_containing_non_alphanumeric_characters_are_invalid() {
        let mut rng = rand::thread_rng();

        // Generate 29 alphanumeric characters
        let alphanumeric_part: String = std::iter::repeat(())
            .map(|_| rng.sample(Alphanumeric) as char)
            .take(29) // 29 characters
            .collect();

        // Generate 1 non-alphanumeric character
        let non_alphanumeric_part: char = match rng.gen_range(0..4) {
            0 => '!', // Example of a special character
            1 => '@',
            2 => '#',
            _ => '$',
        };

        // Combine the parts and shuffle
        let mut token: String = alphanumeric_part + non_alphanumeric_part.to_string().as_str();

        // Shuffle the resulting string to randomize the position of the non-alphanumeric character
        let mut chars: Vec<char> = token.chars().collect();
        chars.shuffle(&mut rng);
        token = chars.iter().collect();

        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn tokens_containing_only_alphanumeric_characters_is_valid() {
        let mut rng = rand::thread_rng();

        // Generate 29 alphanumeric characters
        let token: String = std::iter::repeat(())
            .map(|_| rng.sample(Alphanumeric) as char)
            .take(29) // 29 characters
            .collect();

        assert_ok!(SubscriptionToken::parse(token));
    }
}
