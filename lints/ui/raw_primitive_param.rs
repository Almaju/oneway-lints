pub struct UserId(u64);

pub fn bad(from: u64, to: u64, amount: f64) {
    let _ = (from, to, amount);
}

pub fn good(user_id: UserId) {
    let _ = user_id;
}

fn main() {}
