pub struct UserId(u64);
pub struct AccountId(u64);
pub struct Database;

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

fn main() {}
