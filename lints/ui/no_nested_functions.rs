pub fn outer() {
    fn inner() -> i32 {
        42
    }
    let _ = inner();
}

fn main() {}
