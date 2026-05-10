use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::ffi::OsString;
use std::path::PathBuf;

fn main() -> Result<()> {
    let out_path = parse_out_arg(std::env::args_os().skip(1))?.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/config.schema.json")
    });
    codex_config::schema::write_config_schema(&out_path)?;
    Ok(())
}

fn parse_out_arg(args: impl IntoIterator<Item = OsString>) -> Result<Option<PathBuf>> {
    let mut args = args.into_iter();
    let Some(flag) = args.next() else {
        return Ok(None);
    };
    let flag_display = flag.to_string_lossy();
    if flag_display != "--out" && flag_display != "-o" {
        bail!("unexpected argument {flag:?}; expected --out <PATH>");
    }
    let out_path = args
        .next()
        .map(PathBuf::from)
        .context("missing path after --out")?;
    if let Some(extra) = args.next() {
        bail!("unexpected extra argument {extra:?}");
    }
    Ok(Some(out_path))
}
