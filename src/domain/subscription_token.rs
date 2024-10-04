#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(s: String) -> Result<SubscriptionToken, String> {
        if validate_token(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is no a valid subscription token", s))
        }
    }

    pub fn new(s: &str) -> Self {
        SubscriptionToken(s.to_string())
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn validate_token(token: &str) -> bool {
    // alphanumeric characters only
    let alphanumeric_token = is_alphanumeric(token);
    // length between 20 and 40
    let correct_length = token.len() > 20 && token.len() < 40;

    return alphanumeric_token && correct_length;
}

fn is_alphanumeric(token: &str) -> bool {
    token.chars().all(|c| c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriptionToken;
    use claim::{assert_err, assert_ok};
    use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};

    #[test]
    fn empty_token_is_invalid() {
        assert_err!(SubscriptionToken::parse("".to_string()));
    }

    #[test]
    fn short_token_is_invalid() {
        let token = "a".repeat(20);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn long_token_is_invalid() {
        let token = "a".repeat(40);
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
