pub struct UserId(u64);

pub fn bad(from: u64, to: u64, amount: f64) {
    let _ = (from, to, amount);
}

pub fn good(user_id: UserId) {
    let _ = user_id;
}

pub fn snake_case_name(retry_count: u32) {
    let _ = retry_count;
}

pub fn ref_param(label: &str) {
    let _ = label;
}

pub struct Service;

impl Service {
    pub fn method(&self, count: u32) {
        let _ = count;
    }
}

fn main() {}
