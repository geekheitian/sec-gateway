//! 中国居民身份证号检测器
//!
//! 本模块提供中国身份证号的正则检测功能，支持 18 位身份证号格式验证。

use super::{PIIMatch, PIIType};
use regex::Regex;

/// 中国身份证号正则表达式
const CHINESE_ID_PATTERN: &str =
    r"\b[1-9]\d{5}(18|19|20)\d{2}((0[1-9])|(1[0-2]))(([0-2][1-9])|10|20|30|31)\d{3}[0-9Xx]\b";

/// 检测文本中的中国身份证号
///
/// # 参数
/// - `text`: 待检测的文本内容
///
/// # 返回值
/// 返回 `Vec<(usize, usize, String)>`，每个元素包含:
/// - `usize`: 匹配的起始位置
/// - `usize`: 匹配的结束位置
/// - `String`: 匹配到的身份证号文本
pub fn detect_chinese_id(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(CHINESE_ID_PATTERN).expect("Failed to compile Chinese ID regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::ChineseID,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                1.0,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_valid_id() {
        let text = "ID:110101199001011234";
        let results = detect_chinese_id(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "110101199001011234");
        assert_eq!(results[0].pii_type, PIIType::ChineseID);
    }

    #[test]
    fn test_detect_multiple_ids() {
        let text = "ID1:110101199001011234, ID2:110101199001011235";
        let results = detect_chinese_id(text);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_detect_no_id() {
        let text = "No ID here 12345";
        let results = detect_chinese_id(text);
        assert!(results.is_empty());
    }

    #[test]
    fn test_detect_with_x() {
        let text = "ID:11010119900101123X";
        let results = detect_chinese_id(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "11010119900101123X");
    }
}
