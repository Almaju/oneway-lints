pub fn classify(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

pub fn allowed(opt: Option<i32>) -> i32 {
    if let Some(x) = opt {
        x
    } else {
        0
    }
}

pub fn no_else(x: bool) {
    if x {
        let _ = 1;
    }
}

fn main() {}
