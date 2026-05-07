use std::env;
use std::ffi::OsString;
use std::process::Command;

fn main() {
    if env::var("PROFILE").is_ok_and(|profile| profile.eq_ignore_ascii_case("debug")) {
        eprintln!("Build only release!");
        std::process::exit(1);
    }

    let mut args = env::args_os();
    let _wrapper = args.next();
    let Some(rustc) = args.next() else {
        eprintln!("Build only release! rustc wrapper received no rustc path.");
        std::process::exit(1);
    };
    let rustc_args: Vec<OsString> = args.collect();

    if rustc_args.iter().any(is_debug_profile_arg) {
        eprintln!("Build only release!");
        std::process::exit(1);
    }

    let inner_wrapper = env::var_os("CODEX_CARGO_INNER_RUSTC_WRAPPER")
        .filter(|value| !value.is_empty());

    let status = match inner_wrapper {
        Some(wrapper) => Command::new(wrapper).arg(rustc).args(rustc_args).status(),
        None => Command::new(rustc).args(rustc_args).status(),
    };

    let status = match status {
        Ok(status) => status,
        Err(err) => {
            eprintln!("Build only release! failed to invoke rustc: {err}");
            std::process::exit(1);
        }
    };

    std::process::exit(status.code().unwrap_or(1));
}

fn is_debug_profile_arg(arg: &OsString) -> bool {
    let arg = arg.to_string_lossy().replace('\\', "/");
    arg.contains("/target/debug/") || arg.ends_with("/target/debug")
}
