use std::sync::Mutex;
use zaroxi_domain_ai::conversation::{
    ChatMessage, ChatRole, Conversation, ConversationSession, ConversationStatus,
};

/// Thread-safe manager for conversation sessions.
pub struct SessionManager {
    session: Mutex<ConversationSession>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self { session: Mutex::new(ConversationSession::default()) }
    }

    pub fn active_conversation(&self) -> Conversation {
        self.session.lock().unwrap().active_conversation.clone()
    }

    pub fn active_conversation_mut(&self) -> std::sync::MutexGuard<'_, ConversationSession> {
        self.session.lock().unwrap()
    }

    pub fn new_chat(&self) {
        self.session.lock().unwrap().new_chat();
    }

    pub fn send_message(&self, content: &str) -> ChatMessage {
        let msg = ChatMessage::user(content);
        let mut session = self.session.lock().unwrap();
        session.active_conversation.add_message(msg.clone());
        session.active_conversation.add_message(ChatMessage::assistant(""));
        session.active_conversation.status = ConversationStatus::Sending;
        msg
    }

    pub fn append_streaming_chunk(&self, content: &str) {
        let mut session = self.session.lock().unwrap();
        let conv = &mut session.active_conversation;
        conv.status = ConversationStatus::Streaming;
        if let Some(last) = conv.last_message_mut()
            && last.role == ChatRole::Assistant
        {
            last.append_chunk(content);
            return;
        }
        let mut new_msg = ChatMessage::assistant("");
        new_msg.append_chunk(content);
        conv.add_message(new_msg);
    }

    pub fn finish_streaming(&self) {
        let mut session = self.session.lock().unwrap();
        if let Some(last) = session.active_conversation.last_message_mut() {
            last.finish();
        }
        session.active_conversation.status = ConversationStatus::Idle;
    }

    pub fn set_error(&self, msg: &str) {
        let mut session = self.session.lock().unwrap();
        session.active_conversation.status = ConversationStatus::Error;
        session.active_conversation.last_error = Some(msg.to_string());
    }

    pub fn reset(&self) {
        self.session.lock().unwrap().active_conversation.reset();
    }

    pub fn history(&self) -> Vec<Conversation> {
        self.session.lock().unwrap().history.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_idle() {
        let mgr = SessionManager::new();
        let conv = mgr.active_conversation();
        assert_eq!(conv.status, ConversationStatus::Idle);
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn send_message_returns_user_message() {
        let mgr = SessionManager::new();
        let msg = mgr.send_message("hello");
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn send_message_adds_user_and_empty_assistant() {
        let mgr = SessionManager::new();
        mgr.send_message("hello");
        let conv = mgr.active_conversation();
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].role, ChatRole::User);
        assert_eq!(conv.messages[1].role, ChatRole::Assistant);
        assert_eq!(conv.status, ConversationStatus::Sending);
    }

    #[test]
    fn append_streaming_chunk_and_finish() {
        let mgr = SessionManager::new();
        mgr.send_message("query");
        mgr.append_streaming_chunk("response");
        mgr.append_streaming_chunk(" part 2");
        mgr.finish_streaming();

        let conv = mgr.active_conversation();
        assert_eq!(conv.status, ConversationStatus::Idle);
        let last = conv.messages.last().unwrap();
        assert_eq!(last.content, "response part 2");
        assert!(!last.is_streaming);
        assert!(last.timestamp_ms.is_some());
    }

    #[test]
    fn append_streaming_creates_assistant_if_missing() {
        let mgr = SessionManager::new();
        mgr.append_streaming_chunk("direct");
        let conv = mgr.active_conversation();
        let last = conv.messages.last().unwrap();
        assert_eq!(last.role, ChatRole::Assistant);
        assert_eq!(last.content, "direct");
        assert!(last.is_streaming);
    }

    #[test]
    fn set_error_records_message() {
        let mgr = SessionManager::new();
        mgr.set_error("timeout");
        let conv = mgr.active_conversation();
        assert_eq!(conv.status, ConversationStatus::Error);
        assert_eq!(conv.last_error, Some("timeout".into()));
    }

    #[test]
    fn reset_clears_messages() {
        let mgr = SessionManager::new();
        mgr.send_message("test");
        mgr.reset();
        let conv = mgr.active_conversation();
        assert!(conv.messages.is_empty());
        assert_eq!(conv.status, ConversationStatus::Idle);
    }

    #[test]
    fn new_chat_archives_and_history() {
        let mgr = SessionManager::new();
        mgr.send_message("q1");
        mgr.finish_streaming();
        mgr.new_chat();
        assert_eq!(mgr.history().len(), 1);
        let conv = mgr.active_conversation();
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn active_conversation_mut_allows_direct_access() {
        let mgr = SessionManager::new();
        {
            let mut session = mgr.active_conversation_mut();
            session.active_conversation.add_message(ChatMessage::system("sys"));
        }
        let conv = mgr.active_conversation();
        assert_eq!(conv.messages[0].role, ChatRole::System);
    }
}
