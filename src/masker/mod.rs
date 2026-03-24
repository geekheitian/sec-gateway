//! 文本脱敏模块
//!
//! 本模块提供敏感信息的脱敏功能

pub mod replace;

pub use replace::{mask_with_custom_placeholder, mask_with_placeholder};
