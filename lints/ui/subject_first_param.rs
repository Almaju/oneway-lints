pub struct Server;

impl Server {
    pub fn new() -> Self {
        Self
    }
    pub fn handle(&self, request: Request) {
        let _ = request;
    }
    pub fn over_limit(&self, request: Request, response: Response) {
        let _ = (request, response);
    }
    pub fn ping(&self) {}
}

pub struct Request;
pub struct Response;

pub fn ok_zero() {}

pub fn bad_one_param(request: Request) {
    let _ = request;
}

pub trait MyTrait {
    fn trait_method_too_wide(&self, request: Request, response: Response);
}

impl MyTrait for Server {
    fn trait_method_too_wide(&self, request: Request, response: Response) {
        let _ = (request, response);
    }
}

fn main() {}
