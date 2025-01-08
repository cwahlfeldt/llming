use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Default, Clone)]
pub struct PromptContext {
    pub system_message: Option<String>,
    pub user_message: Option<String>, 
    pub history: Vec<Message>,
    capacity: usize,
}

impl PromptContext {
    pub fn new() -> Self {
        Self {
            capacity: 10,
            ..Default::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    #[inline]
    pub fn set_system_message(&mut self, message: impl Into<String>) {
        self.system_message = Some(message.into());
    }

    #[inline]
    pub fn set_user_message(&mut self, message: impl Into<String>) {
        self.user_message = Some(message.into());
    }

    pub fn add_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        if self.history.len() >= self.capacity {
            self.history.remove(0);
        }
        
        self.history.push(Message {
            role: role.into(),
            content: content.into(),
        });
    }

    // Pre-allocates a String with capacity for the full prompt
    pub fn format(&self) -> String {
        let mut capacity = 0;
        
        if let Some(sys) = &self.system_message {
            capacity += sys.len() + 20;
        }
        
        if let Some(user) = &self.user_message {  
            capacity += user.len() + 20;
        }
        
        for msg in &self.history {
            capacity += msg.role.len() + msg.content.len() + 20;
        }

        let mut output = String::with_capacity(capacity);
        
        if let Some(sys) = &self.system_message {
            output.push_str("System: ");
            output.push_str(sys);
            output.push('\n');
        }

        for msg in &self.history {
            output.push_str(&msg.role);
            output.push_str(": ");
            output.push_str(&msg.content);
            output.push('\n');
        }

        if let Some(user) = &self.user_message {
            output.push_str("User: ");
            output.push_str(user);
        }

        output
    }
}
