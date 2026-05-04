fn main() {
    if let Err(error) = marketplace_mcp::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
