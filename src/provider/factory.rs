use super::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAIProvider, Provider,
};

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn build(kind: crate::config::ProviderKind, target_url: String) -> Box<dyn Provider> {
        match kind {
            crate::config::ProviderKind::OpenAI => Box::new(OpenAIProvider::new(target_url)),
            crate::config::ProviderKind::Anthropic => Box::new(AnthropicProvider::new(target_url)),
            crate::config::ProviderKind::Gemini => Box::new(GeminiProvider::new(target_url)),
        }
    }
}
