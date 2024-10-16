use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, thiserror::Error)]
pub enum SubscriberNameError {
    #[error("Subscriber name cannot be empty or whitespace.")]
    EmptyOrWhitespace,
    #[error(
        "Subscriber name is too long (maximum allowed is {} characters).",
        SubscriberName::MAX_LENGTH
    )]
    TooLong,
    #[error("Subscriber name contains forbidden characters: {0}")]
    ContainsForbiddenCharacters(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct SubscriberName(String);

impl SubscriberName {
    const MAX_LENGTH: usize = 256;
    const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

    /// Returns an instance of `SubscriberName` if the input satisfies all
    /// our validation constraints on subscriber names.
    /// It panics otherwise.
    pub fn parse(s: String) -> Result<SubscriberName, SubscriberNameError> {
        if s.trim().is_empty() {
            return Err(SubscriberNameError::EmptyOrWhitespace);
        }
        if s.graphemes(true).count() > SubscriberName::MAX_LENGTH {
            return Err(SubscriberNameError::TooLong);
        }
        if s.chars()
            .any(|g| SubscriberName::FORBIDDEN_CHARACTERS.contains(&g))
        {
            return Err(SubscriberNameError::ContainsForbiddenCharacters(s.clone()));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
#[cfg(test)]
mod tests {
    use super::SubscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }
    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
