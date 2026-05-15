//! Context ranking algorithms.

use super::context::ContextItem;

/// Rank context items by relevance.
pub fn rank_by_relevance(items: &mut [ContextItem]) {
    items
        .sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
}

/// Rank context items by source.
pub fn rank_by_source(items: &mut [ContextItem]) {
    items.sort_by(|a, b| a.source.cmp(&b.source));
}
