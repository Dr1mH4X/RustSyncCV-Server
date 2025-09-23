pub mod user_store;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm, TokenData, errors::Result as JwtResult};
mod user_db;
pub use user_db::UserDB;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户ID
    pub exp: usize,  // 过期时间
}

pub fn verify_jwt(token: &str, secret: &str) -> JwtResult<TokenData<Claims>> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
}

pub fn create_jwt(user_id: &str, exp: usize, secret: &str) -> JwtResult<String> {
    let claims = Claims { sub: user_id.to_owned(), exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))
}

// 用户名密码校验接口
pub fn verify_user(db: &UserDB, username: &str, password: &str) -> bool {
    db.verify(username, password)
}
