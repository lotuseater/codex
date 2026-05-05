fn main() {
    println!("cargo:rerun-if-env-changed=CODEX_LOCAL_BUILD_STAMP");
}
