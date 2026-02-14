fn main() {
    if std::env::var("DASHSCOPE_API_KEY").is_ok() {
        println!("cargo:rustc-cfg=has_dashscope");
    }
}
