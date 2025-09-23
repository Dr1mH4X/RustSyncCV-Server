use std::collections::HashMap;
use std::sync::Mutex;
use crate::auth::user_store::load_users_from_toml;

pub struct UserDB {
    users: Mutex<HashMap<String, String>>, // username -> password
}

impl UserDB {
    pub fn from_toml(path: &str) -> Self {
        let pairs = load_users_from_toml(path);
        let db: HashMap<String, String> = pairs.into_iter().collect();
        UserDB { users: Mutex::new(db) }
    }

    pub fn verify(&self, username: &str, password: &str) -> bool {
        let users = self.users.lock().unwrap();
        users.get(username).map(|p| p == password).unwrap_or(false)
    }
}
