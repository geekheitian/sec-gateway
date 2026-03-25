//! PII 检测器模块
//!
//! 本模块提供各种 PII（个人身份信息）的检测功能

use serde::{Deserialize, Serialize};

pub mod api_key;
pub mod chinese_id;
pub mod email;
pub mod entropy;
pub mod phone;

/// PII类型枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PIIType {
    /// 中国身份证号
    ChineseID,
    /// 手机号
    PhoneNumber,
    /// 邮箱地址
    Email,
    /// API密钥
    APIKey,
    /// API Secret
    APISecret,
    /// AWS Access Key
    AWSAccessKey,
    /// AWS Secret Key
    AWSSecretKey,
    /// GitHub Token
    GitHubToken,
}

impl PIIType {
    /// 获取PII类型的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            PIIType::ChineseID => "chinese_id",
            PIIType::PhoneNumber => "phone_number",
            PIIType::Email => "email",
            PIIType::APIKey => "api_key",
            PIIType::APISecret => "api_secret",
            PIIType::AWSAccessKey => "aws_access_key",
            PIIType::AWSSecretKey => "aws_secret_key",
            PIIType::GitHubToken => "github_token",
        }
    }
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
}
