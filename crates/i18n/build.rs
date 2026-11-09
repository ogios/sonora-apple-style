fn main() {
    println!("cargo:rustc-check-cfg=cfg(sonora_gui)");
    if std::env::var_os("CARGO_FEATURE_GPUI").is_some() {
        println!("cargo:rustc-cfg=sonora_gui");
    }
}
