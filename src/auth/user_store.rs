use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct UserToml {
    pub users: Vec<UserEntry>,
}

#[derive(Debug, Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub password: String,
}

pub fn load_users_from_toml<P: AsRef<Path>>(path: P) -> Vec<(String, String)> {
    let content = fs::read_to_string(path).expect("Failed to read user toml file");
    let user_toml: UserToml = toml::from_str(&content).expect("Failed to parse user toml");
    user_toml.users.into_iter().map(|u| (u.username, u.password)).collect()
}
