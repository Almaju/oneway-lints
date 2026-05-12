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

pub struct WithRef;

impl WithRef {
    pub fn allowed_method(&self) -> Self {
        Self
    }
}

fn main() {
    let _ = Server::new();
    let _ = Server::create();
    let _ = Db::init();
    let _ = WithRef.allowed_method();
}
