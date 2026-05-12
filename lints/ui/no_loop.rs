pub fn a() {
    let xs = [1, 2, 3];
    for x in &xs {
        let _ = x;
    }
}

pub fn b() {
    let mut i = 0;
    while i < 3 {
        i += 1;
    }
}

pub fn c() {
    loop {
        break;
    }
}

fn main() {}
