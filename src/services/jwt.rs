use std::time::SystemTime;
use std::str::FromStr;
use std::str;

use futures::future;
use futures::{Future, IntoFuture};
use futures_cpupool::CpuPool;
use hyper::{Method, Headers};
use hyper::header::{Authorization, Bearer};
use jsonwebtoken::{encode, Header};
use sha3::{Digest, Sha3_256};
use base64::decode;


use models::jwt::{JWT, ProviderOauth};
use models::user::{NewUser, Provider, UpdateUser, Gender, User};
use repos::identities::{IdentitiesRepo, IdentitiesRepoImpl};
use repos::users::{UsersRepo, UsersRepoImpl};
use http::client::ClientHandle;
use config::JWT as JWTConfig;
use config::OAuth;
use config::Config;
use super::types::ServiceFuture;
use super::error::Error;
use repos::types::DbPool;

#[derive(Serialize, Deserialize, Clone)]
struct GoogleID {
  family_name: String,
  name: String,
  picture: String,
  email: String,
  given_name: String,
  id: String,
  hd: String,
  verified_email: bool
}

impl From<GoogleID> for UpdateUser {
    fn from(google_id: GoogleID) -> Self {
        UpdateUser {
            email: google_id.email,
            phone: None,
            first_name: Some(google_id.name),
            last_name: Some(google_id.family_name),
            middle_name:  None,
            gender: Gender::Undefined,
            birthdate: None,
            last_login_at: SystemTime::now(),
        }
    }
}

impl GoogleID {
    fn update_user(&self, user: User) -> UpdateUser {
        let first_name = if user.first_name.is_none() {
            Some(self.name.clone())
        } else {
            user.first_name
        };
        let last_name = if user.last_name.is_none() {
            Some(self.family_name.clone())
        } else {
            user.last_name
        };
        UpdateUser {
            email: self.email.clone(),
            phone: user.phone,
            first_name: first_name,
            last_name: last_name,
            middle_name:  user.middle_name,
            gender: user.gender,
            birthdate: user.birthdate,
            last_login_at: SystemTime::now(),
        }
    }
}


#[derive(Serialize, Deserialize)]
struct GoogleToken
{
  access_token: String,
  token_type: String,
  expires_in: i32
}

#[derive(Serialize, Deserialize, Clone)]
struct FacebookID {
    id: String,
    email: String,
    gender: String,
    first_name: String,
    last_name: String,
    name: String,
}

impl From<FacebookID> for UpdateUser {
    fn from(facebook_id: FacebookID) -> Self {
        UpdateUser {
            email: facebook_id.email,
            phone: None,
            first_name: Some(facebook_id.first_name),
            last_name: Some(facebook_id.last_name),
            middle_name:  None,
            gender: Gender::from_str(facebook_id.gender.as_ref()).unwrap(),
            birthdate: None,
            last_login_at: SystemTime::now(),
        }
    }
}

impl FacebookID {
    fn update_user(&self, user: User) -> UpdateUser {
        let first_name = if user.first_name.is_none() {
            Some(self.first_name.clone())
        } else {
            user.first_name
        };
        let last_name = if user.last_name.is_none() {
            Some(self.last_name.clone())
        } else {
            user.last_name
        };
        let gender = if user.gender == Gender::Undefined {
            Gender::from_str(self.gender.as_ref()).unwrap()
        } else {
            user.gender
        };
        UpdateUser {
            email: self.email.clone(),
            phone: user.phone,
            first_name: first_name,
            last_name: last_name,
            middle_name:  user.middle_name,
            gender: gender,
            birthdate: user.birthdate,
            last_login_at: SystemTime::now(),
        }
    }
}



#[derive(Serialize, Deserialize)]
struct FacebookToken
{
  access_token: String,
  token_type: String,
  expires_in: i32
}

#[derive(Serialize, Deserialize, Debug)]
struct JWTPayload {
    user_email: String,
}

impl JWTPayload {
    fn new<S: Into<String>>(email: S) -> Self {
        Self {
            user_email: email.into(),
        }
    }
}

pub trait JWTService {

    /// Creates new JWT token by email
    fn create_token_email(&self, payload: NewUser) -> ServiceFuture<JWT>;

    /// Creates new JWT token by google
    fn create_token_google(&self, oauth: ProviderOauth) -> ServiceFuture<JWT>;

    /// Creates new JWT token by facebook
    fn create_token_facebook(&self, oauth: ProviderOauth) -> ServiceFuture<JWT>;

}
/// JWT services, responsible for JsonWebToken operations
pub struct JWTServiceImpl <U:'static + UsersRepo + Clone, I: 'static + IdentitiesRepo+ Clone> {
    pub users_repo: U,
    pub ident_repo: I,
    pub http_client: ClientHandle,
    pub google_config: OAuth,
    pub facebook_config: OAuth,
    pub jwt_config: JWTConfig,
}

impl JWTServiceImpl<UsersRepoImpl, IdentitiesRepoImpl> {
    pub fn new(r2d2_pool: DbPool, cpu_pool:CpuPool, http_client: ClientHandle, config: Config) -> Self {
        let users_repo = UsersRepoImpl::new(r2d2_pool.clone(), cpu_pool.clone());
        let ident_repo = IdentitiesRepoImpl::new(r2d2_pool, cpu_pool);
        Self {
            users_repo: users_repo,
            ident_repo: ident_repo,
            http_client: http_client,
            google_config: config.google,
            facebook_config: config.facebook,
            jwt_config: config.jwt,
        }
    }
}
 

impl<U: UsersRepo + Clone, I: IdentitiesRepo + Clone> JWTService for JWTServiceImpl<U, I> {
    /// Creates new JWT token by email
     fn create_token_email(
        &self,
        payload: NewUser,
    ) -> ServiceFuture<JWT> {
        let ident_repo = self.ident_repo.clone();
        let jwt_secret_key = self.jwt_config.secret_key.clone();

        Box::new(
            ident_repo
                .email_provider_exists(payload.email.to_string(), Provider::Email)
                .map_err(Error::from)
                .map(|exists| (exists, payload))
                .and_then(
                    move |(exists, new_user)| -> ServiceFuture<String> {
                        match exists {
                            // email does not exist
                            false => Box::new(future::err(Error::Validate(validation_errors!({"email": ["email" => "Email or password are incorrect"]})))),
                            // email exists, checking password
                            true => {
                                let new_user_clone = new_user.clone();
                                Box::new(
                                    ident_repo
                                        .find_by_email_provider(new_user.email.clone(), Provider::Email)
                                        .map_err(Error::from)
                                        .and_then (move |identity| 
                                            password_verify(identity.user_password.unwrap().clone(), new_user.password.clone())
                                        )
                                        .map(move |verified| (verified, new_user_clone))
                                        .and_then( move |(verified, user)| -> ServiceFuture<String> {
                                                match verified {
                                                    //password not verified
                                                    false => Box::new(future::err(Error::Validate(validation_errors!({"email": ["email" => "Email or password are incorrect"]})))),
                                                    //password verified
                                                    true => Box::new(future::ok(user.email))
                                                }
                                        })
                                )
                            },
                        }
                    }
                )
                .and_then(move |email| {
                    let tokenpayload = JWTPayload::new(email);
                    encode(&Header::default(), &tokenpayload, jwt_secret_key.as_ref())
                        .map_err(|_| Error::Parse(format!("Couldn't encode jwt: {:?}", tokenpayload)))
                        .into_future()
                        .and_then(|t| future::ok(JWT { token: t }))
                })
        )
    }

    /// https://developers.google.com/identity/protocols/OpenIDConnect#validatinganidtoken
    /// Creates new JWT token by google
     fn create_token_google(
        &self,
        oauth: ProviderOauth,
    ) -> ServiceFuture<JWT> {

        let ident_repo = self.ident_repo.clone();
        let ident_repo_clone = self.ident_repo.clone();
        let users_repo = self.users_repo.clone();
        let users_repo_clone = self.users_repo.clone();
        
        let jwt_secret_key = self.jwt_config.secret_key.clone();
        let info_url = self.google_config.info_url.clone();
        let http_client = self.http_client.clone();

        let mut headers = Headers::new();
        headers.set( Authorization ( Bearer {
            token: oauth.token
        }));

        Box::new(
                http_client.request::<GoogleID>(Method::Get, info_url, None, Some(headers))
                .map_err(|e| Error::HttpClient(format!("Failed to receive user info from google. {}", e.to_string())))
                .and_then(move |google_id| {
                    ident_repo
                        .email_provider_exists(google_id.email.clone(), Provider::Google)
                        .map_err(Error::from)
                        .map(|exists| (exists, google_id))
                })
                 .and_then(
                    move |(exists, google_id)| -> ServiceFuture<String>{
                        
                        match exists {
                            // identity email + provider Google doesn't exist
                            false => {
                                Box::new(users_repo
                                    .email_exists(google_id.email.clone())
                                    .map_err(Error::from)
                                    .map(|email_exist| (google_id, email_exist))
                                    .and_then(move |(google_id, email_exist)| ->  ServiceFuture<String>{
                                        match email_exist {
                                        // user doesn't exist, creating user + identity
                                        false => {
                                            let update_user = UpdateUser::from(google_id.clone());
                                            Box::new(
                                            users_repo_clone
                                                .create(update_user)
                                                .map_err(Error::from)
                                                .map(|user| (google_id, user))
                                                .and_then(move |(google_id, user)| {
                                                    ident_repo_clone
                                                        .create(google_id.email, None, Provider::Google, user.id)
                                                        .map_err(Error::from)
                                                        .map(|u| u.user_email)
                                                })
                                            )
                                        },
                                        // user exists, creating identity and filling user info
                                        true => {
                                            Box::new(
                                            users_repo
                                                .find_by_email(google_id.email.clone())
                                                .map_err(Error::from)
                                                .map(|user| (google_id, user))
                                                .and_then(move |(google_id, user)| {
                                                    let update_user = google_id.update_user(user.clone());
                                                    Box::new(
                                                        users_repo_clone.update(user.id, update_user)
                                                        .map_err(Error::from)
                                                        .map(|u| u.email)
                                                    )
                                                }                                                
                                            ))
                                        }

                                    }})
                                )
                            },
                            // User identity email + provider Google exists, returning Email
                            true => Box::new(future::ok(google_id.email)),
                        }
                    }
                )
                .and_then(move |email| {
                    let tokenpayload = JWTPayload::new(email);
                    encode(&Header::default(), &tokenpayload, jwt_secret_key.as_ref())
                        .map_err(|_| Error::Parse(format!("Couldn't encode jwt: {:?}.", tokenpayload)))
                        .into_future()
                        .and_then(|t| future::ok( JWT { token: t }))
                })
        )
    }

    /// https://developers.facebook.com/docs/facebook-login/manually-build-a-login-flow
    /// Creates new JWT token by facebook
     fn create_token_facebook(
        &self,
        oauth: ProviderOauth,
    ) -> ServiceFuture<JWT> {

        let ident_repo = self.ident_repo.clone();
        let ident_repo_clone = self.ident_repo.clone();
        let users_repo = self.users_repo.clone();
        let users_repo_clone = self.users_repo.clone();

        let jwt_secret_key = self.jwt_config.secret_key.clone();
        let info_url = self.facebook_config.info_url.clone();
        let http_client = self.http_client.clone();

        let url = format!("{}?fields=first_name,last_name,gender,email,name&access_token={}", info_url, oauth.token);

        let future =
                http_client.request::<FacebookID>(Method::Get, url, None, None)
                    .map_err(|e| Error::HttpClient(format!("Failed to receive user info from facebook. {}", e.to_string())))
                    .and_then(move |facebook_id| {
                        ident_repo
                            .email_provider_exists(facebook_id.email.clone(), Provider::Facebook)
                            .map_err(Error::from)
                            .map(|exists| (exists, facebook_id))
                    })
                 .and_then(
                    move |(exists, facebook_id)| -> ServiceFuture<String>{
                        
                        match exists {
                            // identity email + provider facebook doesn't exist
                            false => {
                                Box::new(users_repo
                                    .email_exists(facebook_id.email.clone())
                                    .map_err(Error::from)
                                    .map(|email_exist| (facebook_id, email_exist))
                                    .and_then(move |(facebook_id, email_exist)| ->  ServiceFuture<String>{
                                        match email_exist {
                                        // user doesn't exist, creating user + identity
                                        false => {
                                            let update_user = UpdateUser::from(facebook_id.clone());
                                            Box::new(
                                            users_repo_clone
                                                .create(update_user)
                                                .map_err(Error::from)
                                                .map(|user| (facebook_id, user))
                                                .and_then(move |(facebook_id, user)| {
                                                    ident_repo_clone
                                                        .create(facebook_id.email, None, Provider::Facebook, user.id)
                                                        .map_err(Error::from)
                                                        .map(|u| u.user_email)
                                                })
                                            )
                                        },
                                        // user exists, creating identity and filling user info
                                        true => {
                                            Box::new(
                                            users_repo
                                                .find_by_email(facebook_id.email.clone())
                                                .map_err(Error::from)
                                                .map(|user| (facebook_id, user))
                                                .and_then(move |(facebook_id, user)| {
                                                    let update_user = facebook_id.update_user(user.clone());
                                                    Box::new(
                                                        users_repo_clone.update(user.id, update_user)
                                                        .map_err(Error::from)
                                                        .map(|u| u.email)
                                                    )
                                                }                                                
                                            ))
                                        }

                                    }})
                                )
                            },
                            // User identity email + provider facebook exists, returning Email
                            true => Box::new(future::ok(facebook_id.email)),
                        }
                    }
                )
                .and_then(move |email| {
                    let tokenpayload = JWTPayload::new(email);
                    encode(&Header::default(), &tokenpayload, jwt_secret_key.as_ref())
                        .map_err(|_| Error::Parse(format!("Couldn't encode jwt: {:?}.", tokenpayload)))
                        .into_future()
                        .and_then(|t| future::ok( JWT { token: t }))
                });

        Box::new(future)
    }
}

fn password_verify(db_hash: String, clear_password: String) -> Result<bool, Error> {
    let v: Vec<&str> = db_hash.split('.').collect();
    if v.len() != 2 {
        Err(Error::Validate(validation_errors!({"password": ["password" => "Password in db has wrong format"]})))
    } else {
        let salt = v[1];
        let pass = clear_password + salt;
        let mut hasher = Sha3_256::default();
        hasher.input(pass.as_bytes());
        let out = hasher.result();
        let computed_hash = decode(v[0])
           .map_err(|_| Error::Validate(validation_errors!({"password": ["password" => "Password in db has wrong format"]})))?;
        Ok(computed_hash == &out[..])
    }
}