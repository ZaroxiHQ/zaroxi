#![doc = "AI context collection.\n\nThis module contains domain-first data structures for representing\npieces of contextual information used by AI services. The code in this\ncrate must remain purely domain logic (no RPC/transport concerns)."]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A piece of context that can be used to inform AI decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique identifier for this context item.
    pub id: Uuid,
    /// The content of the context.
    pub content: String,
    /// The source of the context (e.g., \"file\", \"buffer\", \"clipboard\").
    pub source: String,
    /// A relevance score (higher means more relevant).
    pub relevance: f32,
}

impl ContextItem {
    /// Create a new ContextItem.
    pub fn new(id: Uuid, content: impl Into<String>, source: impl Into<String>, relevance: f32) -> Self {
        Self {
            id,
            content: content.into(),
            source: source.into(),
            relevance,
        }
    }
}

/// A collection of context items.
///
/// This collection is intentionally lightweight. It provides helpers that
/// are useful for the service layer (ranking, trimming) while keeping no
/// transport dependencies.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ContextCollection {
    /// The items in the collection.
    pub items: Vec<ContextItem>,
}

impl ContextCollection {
    /// Create a new empty context collection.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a context item to the collection.
    pub fn add(&mut self, item: ContextItem) {
        self.items.push(item);
    }

    /// Merge another collection into this one.
    ///
    /// Items from `other` are appended; callers should deduplicate if needed.
    pub fn merge(&mut self, other: ContextCollection) {
        self.items.extend(other.items);
    }

    /// Get the number of items in the collection.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Return the top `n` items by relevance (descending).
    ///
    /// This method does not reorder the underlying storage; it returns a
    /// new Vec with cloned items. Use this in service code when preparing
    /// a prompt or packing context.
    pub fn top_n_by_relevance(&self, n: usize) -> Vec<ContextItem> {
        let mut items = self.items.clone();
        items.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        items.truncate(n);
        items
    }

    /// Trim the collection in-place to at most `max_items`, keeping the most relevant ones.
    pub fn retain_most_relevant(&mut self, max_items: usize) {
        if self.items.len() <= max_items {
            return;
        }
        self.items.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        self.items.truncate(max_items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn top_n_returns_most_relevant() {
        let mut c = ContextCollection::new();
        c.add(ContextItem::new(Uuid::new_v4(), "low".to_string(), "src".to_string(), 0.1));
        c.add(ContextItem::new(Uuid::new_v4(), "mid".to_string(), "src".to_string(), 0.5));
        c.add(ContextItem::new(Uuid::new_v4(), "high".to_string(), "src".to_string(), 0.9));

        let top2 = c.top_n_by_relevance(2);
        assert_eq!(top2.len(), 2);
        assert!(top2[0].relevance >= top2[1].relevance);
    }

    #[test]
    fn retain_most_relevant_trims() {
        let mut c = ContextCollection::new();
        for i in 0..10 {
            c.add(ContextItem::new(Uuid::new_v4(), format!("item{}", i), "src", i as f32));
        }
        c.retain_most_relevant(3);
        assert_eq!(c.len(), 3);
        assert!(c.items.iter().all(|it| it.relevance >= 7.0));
    }
}
