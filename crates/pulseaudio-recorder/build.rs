fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_pulseaudio)");
    println!("cargo:rerun-if-env-changed=PULSE_SERVER");
    println!("cargo:rerun-if-env-changed=PULSE_RUNTIME_PATH");
    println!("cargo:rerun-if-env-changed=XDG_RUNTIME_DIR");

    if pulseaudio::socket_path_from_env().is_some() {
        println!("cargo:rustc-cfg=has_pulseaudio");
    }
}
