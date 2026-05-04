fn main() {
    if let Err(error) = marketplace_server::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
