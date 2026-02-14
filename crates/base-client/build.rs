fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("missing repo root")?
        .to_path_buf();
    let proto_dir = repo_root.join("proto");
    let proto = proto_dir.join("dictype.proto");
    println!("cargo:rerun-if-changed={}", proto.display());
    tonic_prost_build::configure()
        .build_client(false)
        .build_server(true)
        .compile_protos(&[proto], &[proto_dir])?;
    Ok(())
}
