pub struct Server;

impl Server {
    pub fn new() -> Self {
        Self
    }
    pub fn create() -> Self {
        Self
    }
}

pub struct Db;

impl Db {
    pub fn init() -> Self {
        Self
    }
}

pub struct Cache;

impl Cache {
    pub fn build() -> Self {
        Self
    }
    pub fn construct() -> Self {
        Self
    }
}

pub struct WithRef;

impl WithRef {
    pub fn allowed_method(&self) -> Self {
        Self
    }
}

pub struct UserId(u64);

impl UserId {
    pub fn from_string() -> Self {
        Self(0)
    }
    pub fn new() -> Self {
        Self(0)
    }
}

fn main() {
    let _ = Server::new();
    let _ = Server::create();
    let _ = Db::init();
    let _ = Cache::build();
    let _ = Cache::construct();
    let _ = WithRef.allowed_method();
    let _ = UserId::new();
    let _ = UserId::from_string();
}
