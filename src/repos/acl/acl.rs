//! Authorization module contains authorization logic for the repo layer app
use std::cell::RefCell;
use std::rc::Rc;

use std::collections::HashMap;

use repos::user_roles::UserRolesRepo;
use models::authorization::*;
use super::CachedRoles;

macro_rules! permission {
    ($resource: expr) => { Permission { resource: $resource, action: Action::All, scope: Scope::All }  };
    ($resource: expr, $action: expr) => { Permission { resource: $resource, action: $action, scope: Scope::All }  };
    ($resource: expr, $action: expr, $scope: expr) => { Permission { resource: $resource, action: $action, scope: $scope }  };
}

/// Access control layer for repos. It tells if a user can do a certain action with
/// certain resource. All logic for roles and permissions should be hardcoded into implementation
/// of this trait.
pub trait Acl {
    /// Tells if a user with id `user_id` can do `action` on `resource`.
    /// `resource_with_scope` can tell if this resource is in some scope, which is also a part of `acl` for some
    /// permissions. E.g. You can say that a user can do `Create` (`Action`) on `Store` (`Resource`) only if he's the
    /// `Owner` (`Scope`) of the store.
    fn can (&mut self, resource: Resource, action: Action, resources_with_scope: Vec<&WithScope>) -> bool;
}

#[derive(Clone)]
pub struct SystemAcl {}

impl SystemAcl {
    pub fn new() -> Self {
        Self{}
    }
}

#[allow(unused)]
impl Acl for SystemAcl {
    fn can (&mut self, resource: Resource, action: Action, resources_with_scope: Vec<&WithScope>) -> bool {
        true
    }
}


// TODO: remove info about deleted user from cache
#[derive(Clone)]
pub struct AclImpl<U: UserRolesRepo + 'static + Clone> {
    acls: Rc<RefCell<HashMap<Role, Vec<Permission>>>>,
    cached_roles: CachedRoles<U>,
    user_id: i32
}

macro_rules! hashmap(
    { $($key:expr => $value:expr),+, } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);


impl<U: UserRolesRepo + 'static + Clone> AclImpl<U> {
    pub fn new(cached_roles: CachedRoles<U>, user_id: i32) -> Self {
        let hash = hashmap! {
                Role::Superuser => vec![
                    permission!(Resource::Users), 
                    permission!(Resource::UserRoles)],
                Role::User => vec![
                    permission!(Resource::Users, Action::Read), 
                    permission!(Resource::Users, Action::All, Scope::Owned),
                    permission!(Resource::UserRoles, Action::Read, Scope::Owned)],
        };

        Self { 
            acls: Rc::new(RefCell::new(hash)), 
            cached_roles: cached_roles, 
            user_id: user_id 
        }
    }
}

impl<U: UserRolesRepo + 'static + Clone> Acl for AclImpl<U> {
    fn can(&mut self, resource: Resource, action: Action, resources_with_scope: Vec<&WithScope>) -> bool {
        let empty: Vec<Permission> = Vec::new();
        let user_id = &self.user_id;
        let roles = self.cached_roles.get(*user_id);
        let hashed_acls = self.acls.borrow_mut();
        let acls = roles.into_iter()
            .flat_map(|role| hashed_acls.get(&role).unwrap_or(&empty))
            .filter(|permission|
                (permission.resource == resource) &&
                ((permission.action == action) || (permission.action == Action::All))
            )
            .filter(|permission| resources_with_scope.iter().all(|res| res.is_in_scope(&permission.scope, *user_id)));

        acls.count() > 0
    }
}