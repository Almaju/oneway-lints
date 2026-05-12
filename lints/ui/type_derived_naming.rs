pub struct UserId(u64);
pub struct AccountId(u64);
pub struct Database;

pub trait Migrator {}
pub trait Connector {}
pub trait Orchestrator {}

pub fn good(user_id: UserId, database: Database) {
    let _ = (user_id, database);
}

pub fn good_with_prefix(sender_account_id: AccountId, receiver_account_id: AccountId) {
    let _ = (sender_account_id, receiver_account_id);
}

pub fn bad_param(id: UserId, db: Database) {
    let _ = (id, db);
}

pub fn local_bindings() {
    let user_id: UserId = UserId(1);
    let id: UserId = UserId(2);
    let _ = (user_id, id);
}

pub fn unbound_generic_passes<T>(value: T) {
    let _ = value;
}

pub fn auto_trait_only_passes<T: Send + Sync>(value: T) {
    let _ = value;
}

pub fn single_bound_good<M: Migrator>(migrator: M) {
    let _ = migrator;
}

pub fn single_bound_bad<M: Migrator>(m: M) {
    let _ = m;
}

pub fn multi_bound_placeholder_bad<M: Migrator + Connector + Orchestrator>(m: M) {
    let _ = m;
}

pub fn multi_bound_role_named<Service: Migrator + Connector + Orchestrator>(service: Service) {
    let _ = service;
}

pub fn multi_bound_binding_mismatch<Service: Migrator + Connector>(thing: Service) {
    let _ = thing;
}

pub struct AdminSource;
pub struct Holder {
    pub src: AdminSource,
}

impl Holder {
    pub fn from_admin(source: AdminSource) -> Self {
        Self { src: source }
    }
}

pub struct ShorthandHolder {
    pub source: AdminSource,
}

impl ShorthandHolder {
    pub fn from_admin(source: AdminSource) -> Self {
        Self { source }
    }
}

fn main() {}
