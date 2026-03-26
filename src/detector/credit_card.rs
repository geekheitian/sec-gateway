use super::{PIIMatch, PIIType};

fn luhn_valid(number: &str) -> bool {
    let mut sum = 0;
    let mut alternate = false;

    for ch in number.chars().rev() {
        let mut digit = match ch.to_digit(10) {
            Some(d) => d,
            None => return false,
        };

        if alternate {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }

        sum += digit;
        alternate = !alternate;
    }

    sum % 10 == 0
}

pub fn detect_credit_card(text: &str) -> Vec<PIIMatch> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();

    for start in 0..chars.len() {
        for len in [13usize, 14, 15, 16, 19] {
            let end = start + len;
            if end > chars.len() {
                continue;
            }
            let candidate: String = chars[start..end].iter().collect();
            if !candidate.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let prev_ok = start == 0 || !chars[start - 1].is_ascii_digit();
            let next_ok = end == chars.len() || !chars[end].is_ascii_digit();
            if !prev_ok || !next_ok {
                continue;
            }
            if !luhn_valid(&candidate) {
                continue;
            }

            results.push(PIIMatch::new(
                PIIType::CreditCard,
                candidate,
                start,
                end,
                0.98,
            ));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_credit_card() {
        let text = "Card: 4111111111111111";
        let results = detect_credit_card(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "4111111111111111");
        assert_eq!(results[0].pii_type, PIIType::CreditCard);
    }

    #[test]
    fn test_reject_invalid_card() {
        let text = "Card: 4111111111111112";
        let results = detect_credit_card(text);
        assert!(results.is_empty());
    }

    #[test]
    fn test_detect_card_with_separators() {
        let text = "Card: 4111 1111 1111 1111";
        let results = detect_credit_card(text);
        assert!(results.is_empty());
    }
}
