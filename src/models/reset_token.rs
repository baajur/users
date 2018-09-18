//! Models for password reset
use std::fmt;
use std::time::SystemTime;

use validator::Validate;

use stq_static_resources::TokenType;

use schema::reset_tokens;

#[derive(Serialize, Deserialize, Queryable, Insertable, Debug)]
#[table_name = "reset_tokens"]
pub struct ResetToken {
    pub token: String,
    pub email: String,
    pub created_at: SystemTime,
    pub token_type: TokenType,
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct ResetRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct ResetApply {
    pub token: String,
    #[validate(length(min = "8", max = "30", message = "Password should be between 8 and 30 symbols"))]
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResetMail {
    pub to: String,
    pub subject: String,
    pub text: String,
}

impl fmt::Display for ResetApply {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ResetApply {{ token: \"{}\", password: \"*****\" }}", self.token)
    }
}