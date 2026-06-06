fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc binary if available (avoids system dependency)
    if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_build::compile_protos("proto/benchmark.proto")?;
    Ok(())
}
