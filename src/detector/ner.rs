use serde::{Deserialize, Serialize};

use super::{PIIMatch, PIIType};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NEREntityType {
    Person,
    Location,
    Organization,
}

impl NEREntityType {
    pub fn as_pii_type(&self) -> PIIType {
        match self {
            NEREntityType::Person => PIIType::NERPerson,
            NEREntityType::Location => PIIType::NERLocation,
            NEREntityType::Organization => PIIType::NEROrganization,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NERMatch {
    pub entity_type: NEREntityType,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

impl NERMatch {
    pub fn to_pii_match(&self) -> PIIMatch {
        PIIMatch::new(
            self.entity_type.as_pii_type(),
            self.text.clone(),
            self.start,
            self.end,
            self.confidence,
        )
    }
}

pub struct NERDetector;

impl NERDetector {
    pub fn new() -> Result<Self, NERError> {
        Ok(Self)
    }

    pub fn detect(&self, _text: &str) -> Vec<NERMatch> {
        vec![]
    }

    pub fn detect_to_pii_matches(&self, _text: &str) -> Vec<PIIMatch> {
        vec![]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NERError {
    #[error("NER not implemented yet")]
    NotImplemented,
}
