pub struct Server;

impl Server {
    /// WHY: constructor-style associated function with multiple inputs — the
    /// instance doesn't exist yet so `self` can't be first. Should not be
    /// flagged.
    pub fn connect(name: Name, config: Config) -> Result<Self, ServerError> {
        let _ = (name, config);
        Ok(Self)
    }
    pub fn new() -> Self {
        Self
    }
    /// WHY: fallible constructor returning `Option<Self>` — same carve-out.
    pub fn parse(text: Text) -> Option<Self> {
        let _ = text;
        Some(Self)
    }
    pub fn handle(&self, request: Request) {
        let _ = request;
    }
    pub fn over_limit(&self, request: Request, response: Response) {
        let _ = (request, response);
    }
    pub fn ping(&self) {}
}

pub struct Config;
pub struct Name;
pub struct Request;
pub struct Response;
pub struct ServerError;
pub struct Text;

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
