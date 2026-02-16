use serde::Serialize;

const MAX_HISTORY_TURNS: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ConversationHistory {
    messages: Vec<Message>,
}

impl Default for ConversationHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add_user(&mut self, content: String) {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
        self.trim();
    }

    pub fn add_assistant(&mut self, content: String) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
        });
        self.trim();
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    fn trim(&mut self) {
        let max_messages = MAX_HISTORY_TURNS * 2;
        if self.messages.len() > max_messages {
            let drain_count = self.messages.len() - max_messages;
            self.messages.drain(..drain_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_adds_messages() {
        let mut h = ConversationHistory::new();
        h.add_user("state1".to_string());
        h.add_assistant("cmd1".to_string());
        assert_eq!(h.messages().len(), 2);
    }

    #[test]
    fn history_trims_old_messages() {
        let mut h = ConversationHistory::new();
        for i in 0..50 {
            h.add_user(format!("state{i}"));
            h.add_assistant(format!("cmd{i}"));
        }
        assert_eq!(h.messages().len(), MAX_HISTORY_TURNS * 2);
        assert!(h.messages().last().unwrap().content.contains("cmd49"));
    }
}
