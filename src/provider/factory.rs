use super::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAIProvider, Provider,
    ProviderError,
};

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn build(
        kind: crate::config::ProviderKind,
        target_url: String,
    ) -> Result<Box<dyn Provider>, ProviderError> {
        match kind {
            crate::config::ProviderKind::OpenAI => Ok(Box::new(OpenAIProvider::new(target_url)?)),
            crate::config::ProviderKind::Anthropic => {
                Ok(Box::new(AnthropicProvider::new(target_url)?))
            }
            crate::config::ProviderKind::Gemini => Ok(Box::new(GeminiProvider::new(target_url)?)),
        }
    }
}
