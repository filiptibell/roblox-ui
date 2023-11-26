use rbx_dom_weak::{types::Ref, Instance};
use strsim::normalized_levenshtein;

use super::InstanceMetadata;

pub const QUERY_LIMIT_DEFAULT: usize = 20;
pub const QUERY_LIMIT_MAXIMUM: usize = 100;

#[derive(Debug, Clone)]
pub struct DomQueryParams {
    pub minimum_score: f64,
    pub skip_non_files: bool,
    pub skip_packages: bool,
    pub class_name: Option<String>,
    pub query_low: Option<String>,
    pub limit: Option<usize>,
}

impl DomQueryParams {
    pub fn from_str(query_string: &str) -> Self {
        let query_low = query_string.to_ascii_lowercase();
        Self {
            query_low: Some(query_low),
            ..Default::default()
        }
    }

    pub fn limit(&self) -> usize {
        self.limit
            .unwrap_or(QUERY_LIMIT_DEFAULT)
            .max(QUERY_LIMIT_MAXIMUM)
    }

    pub fn score(&self, inst: &Instance, meta: Option<&InstanceMetadata>) -> Option<f64> {
        // NOTE: Metadata scoring is used here for "extra" sorting
        // with instances that we already think match our query, or
        // for skipping instances completely using any extra filters
        // Metadata matching is also slightly faster than scoring
        // so we do that first to get a bit of extra perf
        if let Some(meta_score) = self.metadata_score(meta) {
            if let Some(inst_score) = self.instance_score(inst) {
                return Some(inst_score + meta_score);
            }
        }
        None
    }

    fn instance_score(&self, inst: &Instance) -> Option<f64> {
        if let Some(class_name) = self.class_name.as_deref() {
            if !class_name.eq_ignore_ascii_case(&inst.class) {
                return None; // No class name match, filtered out
            }
        }

        let mut score = 0.0f64;
        if let Some(query) = self.query_low.as_deref() {
            let inst_name_low = inst.name.to_ascii_lowercase();
            if query == inst_name_low {
                // Exact name match should have max score
                score += 1.0;
            } else {
                // Nonexact matches are weighted slightly towards finding exact substrings
                score += 0.75 * normalized_levenshtein(query, &inst_name_low);
                if inst_name_low.contains(query) {
                    // Boost exact substrings for nonexact matches, making the boost
                    // slightly stronger per how long the query is, up to 8 characters
                    score += 0.25f64.min(0.25 * ((query.len() as f64) / 8.0));
                }
            }
        }

        if score >= self.minimum_score {
            Some(score)
        } else {
            None
        }
    }

    fn metadata_score(&self, meta: Option<&InstanceMetadata>) -> Option<f64> {
        // Check for skipping packages flag
        if self.skip_packages && meta.map(|m| m.package.is_some()).unwrap_or_default() {
            return None;
        }

        // Check for skipping non-direct files flag
        if self.skip_non_files
            && !meta
                .and_then(|m| m.paths.as_ref())
                .map(|p| p.file.is_some() || p.file_meta.is_some())
                .unwrap_or_default()
        {
            return None;
        }

        let mut score = 0.0f64;

        // Sort openable files first
        if meta
            .and_then(|m| m.actions.as_ref())
            .map(|a| a.can_open)
            .unwrap_or_default()
        {
            score += 1.5;
        }

        Some(score)
    }
}

impl Default for DomQueryParams {
    fn default() -> Self {
        Self {
            minimum_score: 0.25,
            skip_non_files: true,
            skip_packages: true,
            class_name: None,
            query_low: None,
            limit: None,
        }
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

impl Eq for DomQueryResult {}

impl Ord for DomQueryResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .partial_cmp(&other.score)
            .expect("scores must not be inf or nan")
    }
}

impl PartialOrd for DomQueryResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
