#[derive(Clone, Debug)]
pub struct AIMessage {
    pub sender: String,
    pub content: String,
}

impl AIMessage {
    pub fn user(content: String) -> AIMessage {
        return AIMessage {
            sender: String::from("user"),
            content: content,
        };
    }

    pub fn assistant(content: String) -> AIMessage {
        return AIMessage {
            sender: String::from("assistant"),
            content: content,
        };
    }

    pub fn system(content: String) -> AIMessage {
        return AIMessage {
            sender: String::from("system"),
            content: content,
        };
    }
}
