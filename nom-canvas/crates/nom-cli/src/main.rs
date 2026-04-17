#![deny(unsafe_code)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit_code = nom_cli::run(&args);
    std::process::exit(exit_code);
}
