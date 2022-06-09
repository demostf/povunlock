use clap::Parser;
use povunlock::unlock;
use std::fs;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the source demo
    path: String,
}

fn main() {
    let args = Args::parse();
    let file = fs::read(&args.path).unwrap();
    let output = unlock(&file);
    fs::write("out.dem", output).unwrap();
}
