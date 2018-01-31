use std::time::SystemTime;

use models::types::UserId;

use validator::Validate;

use models::identity::NewIdentity;
use models::authorization::{Scope, WithScope};

use super::Gender;

table! {
    use diesel::sql_types::*;
    users (id) {
        id -> Integer,
        email -> Varchar,
        email_verified -> Bool,
        phone -> Nullable<VarChar>,
        phone_verified -> Bool,
        is_active -> Bool ,
        first_name -> Nullable<VarChar>,
        last_name -> Nullable<VarChar>,
        middle_name -> Nullable<VarChar>,
        gender -> Nullable<VarChar>,
        birthdate -> Nullable<Timestamp>, //
        last_login_at -> Timestamp, //
        created_at -> Timestamp, // UTC 0, generated at db level
        updated_at -> Timestamp, // UTC 0, generated at db level
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Clone)]
pub struct User {
   pub id: UserId,
   pub email: String,
   pub email_verified: bool,
   pub phone: Option<String>,
   pub phone_verified: bool,
   pub is_active: bool ,
   pub first_name: Option<String>,
   pub last_name: Option<String>,
   pub middle_name: Option<String>,
   pub gender: Gender,
   pub birthdate: Option<SystemTime>,
   pub last_login_at: SystemTime,
   pub created_at: SystemTime,
   pub updated_at: SystemTime,
}

/// Payload for creating users
#[derive(Serialize, Deserialize, Insertable, Validate, Clone)]
#[table_name = "users"]
pub struct NewUser {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(phone(message = "Invalid phone format"))]
    pub phone: Option<String>,
    #[validate(length(min = "1", message = "First name must not be empty"))]
    pub first_name: Option<String>,
    #[validate(length(min = "1", message = "Last name must not be empty"))]
    pub last_name: Option<String>,
    #[validate(length(min = "1", message = "Middle name must not be empty"))]
    pub middle_name: Option<String>,
    pub gender: Gender,
    pub birthdate: Option<SystemTime>,
    pub last_login_at: SystemTime,
}

/// Payload for updating users
#[derive(Serialize, Deserialize, Insertable, Validate, AsChangeset)]
#[table_name = "users"]
pub struct UpdateUser {
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub gender: Option<Gender>,
    pub birthdate: Option<SystemTime>,
    pub last_login_at: Option<SystemTime>,
}

impl WithScope for User {
    fn is_in_scope(&self, scope: &Scope, user_id: i32) -> bool {
        match *scope {
            Scope::All => true,
            Scope::Owned => self.id.0 == user_id
        }
    }
}

impl From<NewIdentity> for NewUser {
    fn from(identity: NewIdentity) -> Self {
        NewUser {
            email: identity.email,
            phone: None,
            first_name: None,
            last_name: None,
            middle_name:  None,
            gender: Gender::Undefined,
            birthdate: None,
            last_login_at: SystemTime::now(),
        }
    }
}
