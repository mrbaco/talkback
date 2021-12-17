pub struct User {
    login: String,
    password_hash: String,
}

impl User {
    pub fn new(login: String, password: String) -> User {
        User {
            login,
            password_hash: User::hash(&password),
        }
    }

    pub fn fill(login: String, password_hash: String) -> User {
        User {
            login,
            password_hash,
        }
    }

    pub fn format(&self) -> String {
        format!("{};{}", self.login, self.password_hash)
    }

    pub fn auth(&self, password: String) -> bool {
        self.password_hash == User::hash(&password)
    }

    fn hash(string: &String) -> String {
        format!("{:x}", md5::compute(string.as_bytes()))
    }
}