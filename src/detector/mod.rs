//! PII 检测器模块
//!
//! 本模块提供各种 PII（个人身份信息）的检测功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::DetectorConfig;
use regex::Regex;

pub mod api_key;
pub mod chinese_id;
pub mod credit_card;
pub mod database_connection_string;
pub mod email;
pub mod entropy;
pub mod ip_address;
pub mod jwt;
pub mod ner;
pub mod phone;

pub use registry::active_detectors;

mod registry;

/// PII类型枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PIIType {
    ChineseID,
    CreditCard,
    PhoneNumber,
    Email,
    IPAddress,
    DatabaseConnectionString,
    JWT,
    APIKey,
    APISecret,
    AWSAccessKey,
    AWSSecretKey,
    GitHubToken,
    NERPerson,
    NERLocation,
    NEROrganization,
}

impl PIIType {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "chinese_id" => Some(PIIType::ChineseID),
            "credit_card" => Some(PIIType::CreditCard),
            "phone_number" => Some(PIIType::PhoneNumber),
            "email" => Some(PIIType::Email),
            "ip_address" => Some(PIIType::IPAddress),
            "database_connection_string" => Some(PIIType::DatabaseConnectionString),
            "jwt" => Some(PIIType::JWT),
            "api_key" => Some(PIIType::APIKey),
            "api_secret" => Some(PIIType::APISecret),
            "aws_access_key" => Some(PIIType::AWSAccessKey),
            "aws_secret_key" => Some(PIIType::AWSSecretKey),
            "github_token" => Some(PIIType::GitHubToken),
            "ner_person" => Some(PIIType::NERPerson),
            "ner_location" => Some(PIIType::NERLocation),
            "ner_organization" => Some(PIIType::NEROrganization),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            PIIType::ChineseID => "chinese_id",
            PIIType::CreditCard => "credit_card",
            PIIType::PhoneNumber => "phone_number",
            PIIType::Email => "email",
            PIIType::IPAddress => "ip_address",
            PIIType::DatabaseConnectionString => "database_connection_string",
            PIIType::JWT => "jwt",
            PIIType::APIKey => "api_key",
            PIIType::APISecret => "api_secret",
            PIIType::AWSAccessKey => "aws_access_key",
            PIIType::AWSSecretKey => "aws_secret_key",
            PIIType::GitHubToken => "github_token",
            PIIType::NERPerson => "ner_person",
            PIIType::NERLocation => "ner_location",
            PIIType::NEROrganization => "ner_organization",
        }
    }
}

pub fn detect_custom_patterns(
    text: &str,
    detectors: &HashMap<String, DetectorConfig>,
) -> Vec<PIIMatch> {
    let mut results = Vec::new();

    for (pii_type_key, detector_cfg) in detectors {
        let Some(pii_type) = PIIType::from_str(pii_type_key.as_str()) else {
            continue;
        };

        let Ok(re) = Regex::new(detector_cfg.pattern.as_str()) else {
            continue;
        };

        for m in re.find_iter(text) {
            results.push(PIIMatch::new(
                pii_type.clone(),
                m.as_str().to_string(),
                m.start(),
                m.end(),
                detector_cfg.confidence_threshold,
            ));
        }
    }

    results
}

/// PII检测匹配结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PIIMatch {
    /// PII类型
    pub pii_type: PIIType,
    /// 匹配的原始值
    pub value: String,
    /// 匹配的起始位置（字节偏移）
    pub start: usize,
    /// 匹配的结束位置（字节偏移）
    pub end: usize,
    /// 检测置信度 (0.0-1.0)
    pub confidence: f32,
}

impl PIIMatch {
    /// 创建新的PII匹配结果
    pub fn new(
        pii_type: PIIType,
        value: String,
        start: usize,
        end: usize,
        confidence: f32,
    ) -> Self {
        Self {
            pii_type,
            value,
            start,
            end,
            confidence,
        }
    }

    /// 获取匹配的字节范围
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_type_as_str() {
        assert_eq!(PIIType::ChineseID.as_str(), "chinese_id");
        assert_eq!(PIIType::PhoneNumber.as_str(), "phone_number");
        assert_eq!(PIIType::Email.as_str(), "email");
    }

    #[test]
    fn test_pii_type_from_str() {
        assert_eq!(PIIType::from_str("chinese_id"), Some(PIIType::ChineseID));
        assert_eq!(
            PIIType::from_str("database_connection_string"),
            Some(PIIType::DatabaseConnectionString)
        );
        assert_eq!(PIIType::from_str("unknown_type"), None);
    }

    #[test]
    fn test_pii_match_creation() {
        let match_result = PIIMatch::new(
            PIIType::ChineseID,
            "110101199001011234".to_string(),
            10,
            28,
            1.0,
        );
        assert_eq!(match_result.pii_type, PIIType::ChineseID);
        assert_eq!(match_result.value, "110101199001011234");
        assert_eq!(match_result.start, 10);
        assert_eq!(match_result.end, 28);
        assert_eq!(match_result.confidence, 1.0);
    }

    #[test]
    fn test_pii_match_range() {
        let match_result =
            PIIMatch::new(PIIType::Email, "test@example.com".to_string(), 0, 16, 0.95);
        assert_eq!(match_result.range(), 0..16);
    }

    #[test]
    fn test_detect_custom_patterns() {
        let mut detectors = HashMap::new();
        detectors.insert(
            "email".to_string(),
            DetectorConfig {
                pattern: "[a-zA-Z0-9._%+-]+@example\\.com".to_string(),
                confidence_threshold: 0.77,
            },
        );

        let results = detect_custom_patterns("contact me at alice@example.com", &detectors);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pii_type, PIIType::Email);
        assert_eq!(results[0].value, "alice@example.com");
        assert_eq!(results[0].confidence, 0.77);
    }
}
