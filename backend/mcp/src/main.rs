fn main() {
    if let Err(error) = oz_market_mcp::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
