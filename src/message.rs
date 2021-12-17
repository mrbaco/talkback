pub struct Message {
    id: usize,
    login: String,
    text: String,
}

impl Message {
    pub fn new(id: usize, login: String, text: String) -> Message {
        Message {
            id,
            login,
            text
        }
    }

    pub fn format(&self) -> String {
        format!("<b>{}</b>: {}", self.login, self.text)
    }
}

impl Clone for Message {
    fn clone(&self) -> Self {
        Message {
            id: self.id,
            login: String::from(&self.login),
            text: String::from(&self.text),
        }
    }
}