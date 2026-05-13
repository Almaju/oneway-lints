pub struct UserId(u64);

pub struct Bad {
    pub age: u32,
    pub name: String,
}

pub struct Good {
    pub age: Age,
    pub user_id: UserId,
}

pub struct Age(u32);

struct MixedVis {
    pub created_at: u64,
    retry_count: u32,
}

#[derive(Debug)]
pub struct WithDerive {
    pub session_id: u64,
}

pub struct WithRef<'a> {
    pub label: &'a str,
}

fn main() {}
