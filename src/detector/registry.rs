use super::{
    api_key::detect_api_keys, chinese_id::detect_chinese_id, credit_card::detect_credit_card,
    database_connection_string::detect_database_connection_string, email::detect_email,
    ip_address::detect_ip_address, jwt::detect_jwt, phone::detect_phone_number, PIIMatch,
};

type DetectorFn = fn(&str) -> Vec<PIIMatch>;

fn detector_registry() -> Vec<(&'static str, DetectorFn)> {
    vec![
        ("chinese_id", detect_chinese_id),
        ("phone_number", detect_phone_number),
        ("email", detect_email),
        ("credit_card", detect_credit_card),
        ("jwt", detect_jwt),
        ("api_key", detect_api_keys),
        ("ip_address", detect_ip_address),
        (
            "database_connection_string",
            detect_database_connection_string,
        ),
    ]
}

pub fn active_detectors(enabled_types: &[String]) -> Vec<DetectorFn> {
    let enabled: std::collections::HashSet<&str> =
        enabled_types.iter().map(|t| t.as_str()).collect();

    detector_registry()
        .into_iter()
        .filter(|(pii_type, _)| enabled.contains(pii_type))
        .map(|(_, detect_fn)| detect_fn)
        .collect()
}
