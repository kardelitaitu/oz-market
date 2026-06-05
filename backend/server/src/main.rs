fn main() {
    if let Err(error) = oz_market_server::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
