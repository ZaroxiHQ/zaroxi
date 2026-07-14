//! Conversation and session model for the AI assistant pane.
//!
//! Defines the core data structures for chat conversations: messages with
//! roles, session state, streaming support, and request lifecycle.
//! These are pure domain types — no transport, rendering, or persistence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The role of a participant in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A single message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message identifier.
    pub id: String,
    /// Who sent this message.
    pub role: ChatRole,
    /// The full text content.
    pub content: String,
    /// Whether this message is still being streamed (partial).
    pub is_streaming: bool,
    /// Timestamp (epoch millis) when the message was completed.
    pub timestamp_ms: Option<u64>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::User,
            content: content.into(),
            is_streaming: false,
            timestamp_ms: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: content.into(),
            is_streaming: false,
            timestamp_ms: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: ChatRole::System,
            content: content.into(),
            is_streaming: false,
            timestamp_ms: None,
        }
    }

    /// Append a chunk of text (for streaming).
    pub fn append_chunk(&mut self, chunk: &str) {
        self.content.push_str(chunk);
        self.is_streaming = true;
    }

    /// Mark streaming as complete.
    pub fn finish(&mut self) {
        self.is_streaming = false;
        self.timestamp_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
    }
}

/// The status of the current AI request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationStatus {
    Idle,
    Sending,
    Streaming,
    Error,
    Cancelled,
}

/// A conversation session containing messages and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique session identifier.
    pub id: String,
    /// Human-readable title (auto-generated or user-set).
    pub title: String,
    /// All messages in chronological order.
    pub messages: Vec<ChatMessage>,
    /// Current request status.
    pub status: ConversationStatus,
    /// The last error message, if status is Error.
    pub last_error: Option<String>,
    /// Creation timestamp (epoch millis).
    pub created_at_ms: u64,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Chat".into(),
            messages: Vec::new(),
            status: ConversationStatus::Idle,
            last_error: None,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    /// Get the last message, if any.
    pub fn last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    /// Get the last mutable message (used for streaming append).
    pub fn last_message_mut(&mut self) -> Option<&mut ChatMessage> {
        self.messages.last_mut()
    }

    /// Remove all messages and reset to idle.
    pub fn reset(&mut self) {
        self.messages.clear();
        self.status = ConversationStatus::Idle;
        self.last_error = None;
        self.created_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

/// Aggregate session state for the AI panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationSession {
    /// Current active conversation.
    pub active_conversation: Conversation,
    /// Previously saved conversations (future: session history).
    pub history: Vec<Conversation>,
}

impl Default for ConversationSession {
    fn default() -> Self {
        Self { active_conversation: Conversation::new(), history: Vec::new() }
    }
}

impl ConversationSession {
    /// Start a fresh conversation, archiving the current one.
    pub fn new_chat(&mut self) {
        if !self.active_conversation.messages.is_empty() {
            let old = std::mem::replace(&mut self.active_conversation, Conversation::new());
            self.history.push(old);
        } else {
            self.active_conversation = Conversation::new();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_user_has_correct_role() {
        let msg = ChatMessage::user("hello");
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "hello");
        assert!(!msg.is_streaming);
    }

    #[test]
    fn chat_message_streaming_append() {
        let mut msg = ChatMessage::assistant("He");
        msg.append_chunk("llo");
        assert!(msg.is_streaming);
        assert_eq!(msg.content, "Hello");
        msg.finish();
        assert!(!msg.is_streaming);
        assert!(msg.timestamp_ms.is_some());
    }

    #[test]
    fn conversation_new_is_idle() {
        let conv = Conversation::new();
        assert_eq!(conv.status, ConversationStatus::Idle);
        assert!(conv.messages.is_empty());
        assert_eq!(conv.title, "New Chat");
    }

    #[test]
    fn conversation_add_and_last_message() {
        let mut conv = Conversation::new();
        conv.add_message(ChatMessage::user("query"));
        conv.add_message(ChatMessage::assistant("answer"));
        assert_eq!(conv.last_message().unwrap().content, "answer");
    }

    #[test]
    fn conversation_reset_clears_all() {
        let mut conv = Conversation::new();
        conv.add_message(ChatMessage::user("test"));
        conv.status = ConversationStatus::Error;
        conv.reset();
        assert!(conv.messages.is_empty());
        assert_eq!(conv.status, ConversationStatus::Idle);
        assert!(conv.last_error.is_none());
    }

    #[test]
    fn session_new_chat_archives() {
        let mut session = ConversationSession::default();
        session.active_conversation.add_message(ChatMessage::user("q"));
        session.new_chat();
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.active_conversation.messages.len(), 0);
    }

    #[test]
    fn session_new_chat_no_archive_when_empty() {
        let mut session = ConversationSession::default();
        session.new_chat();
        assert!(session.history.is_empty());
    }
}
