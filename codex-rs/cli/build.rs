fn main() {
    println!("cargo:rerun-if-env-changed=CODEX_LOCAL_BUILD_STAMP");
    // Pipe the ambient stamp through rustc-env so `option_env!` reads a
    // fingerprint-tracked value: without this, cargo never recompiles the
    // crate on a stamp change and the `None` baked on the first compile
    // persists. When the var is absent (normal dev builds), nothing is
    // emitted and `option_env!` correctly yields `None`.
    if let Ok(stamp) = std::env::var("CODEX_LOCAL_BUILD_STAMP") {
        println!("cargo:rustc-env=CODEX_LOCAL_BUILD_STAMP={stamp}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-ObjC");
    }
}
