pub fn display_version(package_version: &str, local_build_stamp: Option<&str>) -> String {
    match local_build_stamp
        .map(str::trim)
        .filter(|stamp| !stamp.is_empty())
    {
        Some(stamp) => format!("{package_version} (local build {stamp})"),
        None => package_version.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_version_uses_package_version_without_local_build_stamp() {
        assert_eq!(display_version("0.0.0", None), "0.0.0");
        assert_eq!(display_version("0.0.0", Some("  ")), "0.0.0");
    }

    #[test]
    fn display_version_appends_local_build_stamp() {
        assert_eq!(
            display_version("0.0.0", Some("2026-05-05T05:45:00+03:00")),
            "0.0.0 (local build 2026-05-05T05:45:00+03:00)"
        );
    }
}
