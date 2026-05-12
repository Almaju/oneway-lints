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

fn main() {}
