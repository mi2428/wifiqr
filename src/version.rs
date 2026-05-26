#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub struct VersionInfo<'a> {
    pub package_name: &'a str,
    pub package_version: &'a str,
    pub git_describe: &'a str,
    pub git_commit: &'a str,
    pub git_commit_date: &'a str,
    pub build_date: &'a str,
    pub build_host: &'a str,
    pub build_target: &'a str,
    pub build_profile: &'a str,
}

pub const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (git ",
    env!("WIFIQR_GIT_DESCRIBE"),
    "; commit ",
    env!("WIFIQR_GIT_COMMIT"),
    "; commit date ",
    env!("WIFIQR_GIT_COMMIT_DATE"),
    "; built ",
    env!("WIFIQR_BUILD_DATE"),
    "; ",
    env!("WIFIQR_BUILD_PROFILE"),
    ") on ",
    env!("WIFIQR_BUILD_TARGET"),
    " (host ",
    env!("WIFIQR_BUILD_HOST"),
    ")"
);

#[cfg(test)]
pub fn current() -> VersionInfo<'static> {
    VersionInfo {
        package_name: env!("CARGO_PKG_NAME"),
        package_version: env!("CARGO_PKG_VERSION"),
        git_describe: env!("WIFIQR_GIT_DESCRIBE"),
        git_commit: env!("WIFIQR_GIT_COMMIT"),
        git_commit_date: env!("WIFIQR_GIT_COMMIT_DATE"),
        build_date: env!("WIFIQR_BUILD_DATE"),
        build_host: env!("WIFIQR_BUILD_HOST"),
        build_target: env!("WIFIQR_BUILD_TARGET"),
        build_profile: env!("WIFIQR_BUILD_PROFILE"),
    }
}

#[cfg(test)]
pub fn render(info: &VersionInfo<'_>) -> String {
    format!(
        "{} {} (git {}; commit {}; commit date {}; built {}; {}) on {} (host {})\n",
        info.package_name,
        info.package_version,
        info.git_describe,
        info.git_commit,
        info.git_commit_date,
        info.build_date,
        info.build_profile,
        info.build_target,
        info.build_host,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_build_metadata() {
        let rendered = render(&VersionInfo {
            package_name: "wifiqr",
            package_version: "0.1.0",
            git_describe: "v0.1.0-1-gabc123",
            git_commit: "abc123",
            git_commit_date: "2026-04-25T00:00:00+09:00",
            build_date: "2026-04-25T01:00:00Z",
            build_host: "aarch64-apple-darwin",
            build_target: "x86_64-unknown-linux-gnu",
            build_profile: "release",
        });

        assert_eq!(
            rendered,
            concat!(
                "wifiqr 0.1.0 ",
                "(git v0.1.0-1-gabc123; commit abc123; ",
                "commit date 2026-04-25T00:00:00+09:00; ",
                "built 2026-04-25T01:00:00Z; release) ",
                "on x86_64-unknown-linux-gnu ",
                "(host aarch64-apple-darwin)\n"
            )
        );
    }

    #[test]
    fn long_version_matches_current_render_without_binary_name() {
        let rendered = render(&current());
        let long_version = rendered
            .strip_prefix(concat!(env!("CARGO_PKG_NAME"), " "))
            .expect("rendered version should start with package name");

        assert_eq!(LONG_VERSION, long_version.trim_end());
    }
}
