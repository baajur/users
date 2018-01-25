use futures::future;
use futures::Future;
use futures_cpupool::CpuPool;

use models::user::{User, NewUser, UpdateUser};
use repos::users::{UsersRepo, UsersRepoImpl};
use super::types::ServiceFuture;
use super::error::Error;
use repos::types::DbPool;


pub trait UsersService {
    /// Returns user by ID
    fn get(&self, user_id: i32) -> ServiceFuture<User>;
    /// Returns current user
    fn current(&self) -> ServiceFuture<User>;
    /// Lists users limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i64) -> ServiceFuture<Vec<User>>;
    /// Deactivates specific user
    fn deactivate(&self, user_id: i32) -> ServiceFuture<User>;
    /// Creates new user
    fn create(&self, payload: NewUser) -> ServiceFuture<User>;
    /// Updates specific user
    fn update(&self, user_id: i32, payload: UpdateUser) -> ServiceFuture<User>;
}

/// Users services, responsible for User-related CRUD operations
pub struct UsersServiceImpl<U: 'static + UsersRepo + Clone> {
    pub users_repo: U,
    pub user_email: Option<String>
}

impl UsersServiceImpl<UsersRepoImpl> {
    pub fn new(r2d2_pool: DbPool, cpu_pool:CpuPool, user_email: Option<String>) -> Self {
        let users_repo = UsersRepoImpl::new(r2d2_pool, cpu_pool);
        Self {
            users_repo: users_repo,
            user_email: user_email
        }
    }
}

impl<U: UsersRepo + Clone> UsersService for UsersServiceImpl<U> {
    /// Returns user by ID
    fn get(&self, user_id: i32) -> ServiceFuture<User> {
        Box::new(self.users_repo.find(user_id).map_err(Error::from))
    }

    /// Returns current user
    fn current(&self) -> ServiceFuture<User>{
        if let Some(ref email) = self.user_email {
            Box::new(self.users_repo.find_by_email(email.to_string()).map_err(Error::from))
        } else {
            Box::new(future::err(Error::Unknown("There is no user email in request header.")))
        }
    }
    
    /// Lists users limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i64) -> ServiceFuture<Vec<User>> {
        Box::new(
            self.users_repo
                .list(from, count)
                .map_err(|e| Error::from(e)),
        )
    }

    /// Deactivates specific user
    fn deactivate(&self, user_id: i32) -> ServiceFuture<User> {
        Box::new(
            self.users_repo
                .deactivate(user_id)
                .map_err(|e| Error::from(e)),
        )
    }

    /// Creates new user
    fn create(&self, payload: NewUser) -> ServiceFuture<User> {
        let users_repo = self.users_repo.clone();
        Box::new(
            users_repo
                .email_exists(payload.email.to_string())
                .map(|exists| (payload, exists))
                .map_err(|e| Error::from(e))
                .and_then(|(payload, exists)| match exists {
                    false => future::ok(payload),
                    true => future::err(Error::Validate(validation_errors!({"email": ["email" => "Email already exists"]})))
                })
                .and_then(move |user| {
                    users_repo.create(user).map_err(|e| Error::from(e))
                }),
        )
    }

    /// Updates specific user
    fn update(&self, user_id: i32, payload: UpdateUser) -> ServiceFuture<User> {
        let users_repo = self.users_repo.clone();

        Box::new(
            users_repo
                .find(user_id)
                .and_then(move |_user| users_repo.update(user_id, payload))
                .map_err(|e| Error::from(e)),
        )
    }
}
