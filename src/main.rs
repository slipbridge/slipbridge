use std::process;

fn main() {
    if let Err(e) = slipbridge::run() {
        eprintln!("slipbridge: {e}");
        process::exit(1);
    }
}
