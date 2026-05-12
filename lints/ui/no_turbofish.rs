pub fn run() {
    let xs = [1u32, 2, 3];
    let _: Vec<u32> = xs.iter().copied().collect();
    let _ = xs.iter().copied().collect::<Vec<u32>>();
    let _: u32 = "42".parse().unwrap_or(0);
    let _ = "42".parse::<u32>().unwrap_or(0);
}

fn main() {}
