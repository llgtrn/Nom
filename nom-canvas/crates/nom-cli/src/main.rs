use nom_cli::parse_args;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = raw.iter().map(String::as_str).collect();
    match parse_args(&refs) {
        Ok(cmd) => println!("nom-canvas: {:?}", cmd),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
