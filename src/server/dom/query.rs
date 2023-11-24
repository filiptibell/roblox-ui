use std::{convert::Infallible, str::FromStr};

use rbx_dom_weak::{types::Ref, Instance};
use strsim::normalized_levenshtein;

const MINIMUM_MATCH_SCORE: f64 = 0.2;

pub const QUERY_LIMIT_DEFAULT: usize = 20;
pub const QUERY_LIMIT_MAXIMUM: usize = 100;

#[derive(Debug, Clone)]
pub(super) struct QueryParams {
    pub name: String,
}

impl QueryParams {
    pub fn score(&self, inst: &Instance) -> Option<f64> {
        let mut score = 0.0f64;
        score += normalized_levenshtein(&self.name, &inst.name);
        if score >= MINIMUM_MATCH_SCORE {
            Some(score)
        } else {
            None
        }
    }
}

impl FromStr for QueryParams {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            name: s.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct QueryResult {
    pub score: f64,
    pub id: Ref,
}

impl QueryResult {
    pub fn new(score: f64, id: Ref) -> Self {
        Self { score, id }
    }
}

impl PartialOrd for QueryResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}
