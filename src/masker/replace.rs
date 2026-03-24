//! 文本脱敏替换模块
//!
//! 本模块提供文本脱敏功能，将敏感信息替换为占位符

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static REDACTION_COUNTER: LazyLock<Mutex<HashMap<String, usize>>> = 
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn next_redaction_id(key: &str) -> usize {
    let mut counters = REDACTION_COUNTER.lock().unwrap();
    let count = counters.get(key).copied().unwrap_or(0) + 1;
    counters.insert(key.to_string(), count);
    count
}

/// 替换文本中的敏感信息为递增编号的占位符
///
/// # 参数
/// - `text`: 原始文本
/// - `detections`: 检测结果，`Vec<(start, end, matched_text)>` 格式
/// - `prefix`: 占位符前缀
/// - `suffix`: 占位符后缀
///
/// # 返回值
/// 返回脱敏后的文本，检测到的敏感信息被替换为 `[REDACTED_ID_001]` 格式的占位符
pub fn mask_with_placeholder(
    text: &str,
    detections: &[(usize, usize, String)],
    prefix: &str,
    suffix: &str,
) -> String {
    let mut sorted_detections: Vec<_> = detections.iter().collect();
    sorted_detections.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result = text.to_string();
    for (start, end, _) in sorted_detections {
        let redaction_id = next_redaction_id("id");
        let placeholder = format!("{}_{:03}{}", prefix, redaction_id, suffix);
        result.replace_range(*start..*end, &placeholder);
    }

    result
}

/// 替换文本中的敏感信息为自定义占位符
///
/// # 参数
/// - `text`: 原始文本
/// - `detections`: 检测结果，`Vec<(start, end, matched_text)>` 格式
/// - `placeholder`: 自定义占位符字符串
///
/// # 返回值
/// 返回脱敏后的文本
pub fn mask_with_custom_placeholder(
    text: &str,
    detections: &[(usize, usize, String)],
    placeholder: &str,
) -> String {
    let mut sorted_detections: Vec<_> = detections.iter().collect();
    sorted_detections.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result = text.to_string();
    for (start, end, _) in sorted_detections {
        result.replace_range(*start..*end, placeholder);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_basic() {
        let text = "ID:110101199001011234";
        let detections = vec![(3, 21, "110101199001011234".to_string())];
        let result = mask_with_placeholder(text, &detections, "[REDACTED_ID", "]");
        assert!(result.starts_with("ID:[REDACTED_ID_"));
        assert!(result.ends_with("]"));
    }

    #[test]
    fn test_mask_multiple() {
        let text = "ID1:110101199001011234, ID2:110101199001011235";
        let detections = vec![
            (4, 22, "110101199001011234".to_string()),
            (28, 46, "110101199001011235".to_string()),
        ];
        let result = mask_with_placeholder(text, &detections, "[REDACTED_ID", "]");
        assert!(result.contains("[REDACTED_ID_"));
        assert!(result.contains("ID2:"));
        let count = result.matches("[REDACTED_ID_").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_mask_custom() {
        let text = "ID:110101199001011234";
        let detections = vec![(3, 21, "110101199001011234".to_string())];
        let result = mask_with_custom_placeholder(text, &detections, "[HIDDEN]");
        assert_eq!(result, "ID:[HIDDEN]");
    }
}
