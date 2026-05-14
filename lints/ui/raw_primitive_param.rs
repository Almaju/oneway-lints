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

pub struct Role;
pub struct UnknownRole;

// WHY: trait impl signatures are fixed by the trait. `FromStr::from_str`
// takes `s: &str` and the user can't change that — the lint must skip
// trait impl methods just like `subject_first_param` does.
impl std::str::FromStr for Role {
    type Err = UnknownRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let _ = s;
        Ok(Self)
    }
}

pub struct RedirectUrl(String);

// WHY: same shape — `From<&str>` is a foreign trait whose `from(s: &str)`
// signature the user can't change. The lint must skip trait impl methods.
impl From<&str> for RedirectUrl {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

fn main() {}
