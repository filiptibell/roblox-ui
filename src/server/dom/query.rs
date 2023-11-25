use rbx_dom_weak::{types::Ref, Instance};
use strsim::normalized_levenshtein;

use super::InstanceMetadata;

pub const QUERY_LIMIT_DEFAULT: usize = 40;
pub const QUERY_LIMIT_MAXIMUM: usize = 100;

#[derive(Debug, Clone)]
pub struct DomQueryParams {
    pub class_name: Option<String>,
    pub name_low: Option<String>,
    pub limit: Option<usize>,
}

impl DomQueryParams {
    pub fn from_str(query_string: &str) -> Self {
        // TODO: Parse out class name from query string
        Self {
            class_name: None,
            name_low: Some(query_string.to_ascii_lowercase()),
            limit: None,
        }
    }

    pub fn limit(&self) -> usize {
        self.limit
            .unwrap_or(QUERY_LIMIT_DEFAULT)
            .max(QUERY_LIMIT_MAXIMUM)
    }

    pub fn score(&self, inst: &Instance, meta: Option<&InstanceMetadata>) -> f64 {
        self.instance_score(inst) + self.metadata_score(meta)
    }

    fn instance_score(&self, inst: &Instance) -> f64 {
        let mut score = 0.0f64;

        if let Some(class_name) = self.class_name.as_deref() {
            if class_name.eq_ignore_ascii_case(&inst.class) {
                score += 1.0; // Exact class name match
            }
        }

        if let Some(name_low) = self.name_low.as_deref() {
            let inst_name_low = inst.name.to_ascii_lowercase();
            if name_low == inst_name_low {
                score += 1.0; // Exact name match
            } else {
                score += normalized_levenshtein(name_low, &inst_name_low);
                if inst_name_low.contains(name_low) {
                    score += 0.2; // Boost exact substrings for nonexact matches
                }
            }
        }

        score
    }

    fn metadata_score(&self, meta: Option<&InstanceMetadata>) -> f64 {
        let mut score = 0.0f64;

        if meta
            .and_then(|m| m.actions.as_ref())
            .map(|a| a.can_open)
            .unwrap_or_default()
        {
            score += 5.0; // Sort openable files first
        }

        if meta.map(|m| m.package.is_some()).unwrap_or_default() {
            score -= 2.5; // Sort packages last
        }

        score
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomQueryResult {
    pub score: f64,
    pub id: Ref,
}

impl DomQueryResult {
    pub fn new(score: f64, id: Ref) -> Self {
        Self { score, id }
    }
}

impl PartialOrd for DomQueryResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}
