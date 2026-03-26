use super::{PIIMatch, PIIType};
use regex::Regex;

const DB_CONNECTION_STRING_PATTERN: &str =
    "\\b(?:postgres(?:ql)?|mysql|mariadb|mongodb|redis|sqlserver)://[^\\s\"']+\\b";

pub fn detect_database_connection_string(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(DB_CONNECTION_STRING_PATTERN)
        .expect("Failed to compile database connection string regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::DatabaseConnectionString,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                0.93,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_postgres_connection_string() {
        let text = "dsn=postgres://user:pass@localhost:5432/mydb";
        let results = detect_database_connection_string(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pii_type, PIIType::DatabaseConnectionString);
        assert_eq!(results[0].value, "postgres://user:pass@localhost:5432/mydb");
    }

    #[test]
    fn test_detect_mysql_connection_string() {
        let text = "mysql://root:secret@127.0.0.1:3306/app";
        let results = detect_database_connection_string(text);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_reject_non_connection_string() {
        let text = "Use mysql and postgres in docs only";
        let results = detect_database_connection_string(text);
        assert!(results.is_empty());
    }
}
