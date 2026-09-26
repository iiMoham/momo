//! MoMo's identity: its command, app directory, release channel, and name in
//! the UI. Compatibility identifiers (`HERDR_*` variables, hook sources, socket
//! file names) keep their original spelling so existing agent hooks and plugins
//! keep working.

/// The command users type.
pub(crate) const CLI_NAME: &str = "momo";
/// The product name shown in prose.
pub(crate) const PRODUCT_NAME: &str = "MoMo";
/// The app directory name for release builds (`~/.config/momo`, state, sockets).
pub(crate) const APP_DIR: &str = "momo";
/// The app directory name for debug builds, so development never touches a
/// release install.
pub(crate) const DEV_APP_DIR: &str = "momo-dev";
/// The upstream app directory MoMo imports a config from on first start.
pub(crate) const UPSTREAM_APP_DIR: &str = "herdr";
/// MoMo's GitHub repository.
pub(crate) const REPOSITORY_URL: &str = "https://github.com/iiMoham/momo";
/// The update manifest attached to MoMo's latest GitHub Release. `momo update`
/// and remote SSH installs read it instead of herdr.dev.
pub(crate) const UPDATE_MANIFEST_URL: &str =
    "https://github.com/iiMoham/momo/releases/latest/download/latest.json";
/// Build channel MoMo release builds set through `MOMO_BUILD_CHANNEL`; the
/// release number goes in `MOMO_BUILD_ID`, so versions read `0.9.1-momo.3`.
pub(crate) const RELEASE_CHANNEL: &str = "momo";
/// MoMo has no preview builds. Upstream's preview code stays compiled behind
/// this flag so merges from herdr stay small.
pub(crate) const HAS_PREVIEW_CHANNEL: bool = false;
/// Why MoMo refuses the preview channel.
pub(crate) const NO_PREVIEW_CHANNEL: &str =
    "MoMo publishes stable releases only; the preview channel is not available (run `momo channel set stable`)";

/// This build's MoMo release number: `N` in `0.9.1-momo.N`, or 0 for local
/// and development builds.
pub(crate) fn release_number() -> u32 {
    release_number_for(crate::build_info::channel(), crate::build_info::build_id())
}

fn release_number_for(channel: &str, build_id: Option<&str>) -> u32 {
    if channel != RELEASE_CHANNEL {
        return 0;
    }
    build_id
        .and_then(|build_id| build_id.parse().ok())
        .unwrap_or(0)
}

/// Display label of a MoMo release: `0.9.1-momo.3`, or the base version for
/// release 0.
pub(crate) fn release_label(base_version: &str, release: u32) -> String {
    if release == 0 {
        base_version.to_owned()
    } else {
        format!("{base_version}-{RELEASE_CHANNEL}.{release}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_numbers_come_only_from_momo_release_builds() {
        assert_eq!(release_number_for("momo", Some("3")), 3);
        assert_eq!(release_number_for("momo", Some("x")), 0);
        assert_eq!(release_number_for("momo", None), 0);
        assert_eq!(release_number_for("stable", Some("3")), 0);
        assert_eq!(release_number_for("preview", Some("3")), 0);
        assert_eq!(release_label("0.9.1", 3), "0.9.1-momo.3");
        assert_eq!(release_label("0.9.1", 0), "0.9.1");
    }

    #[test]
    fn bundled_skill_teaches_momo_commands() {
        let skill = include_str!("../skills/momo/SKILL.md");
        assert!(skill.starts_with("---\nname: momo\n"));
        let herdr_command = regex::Regex::new(r"(^|[^\w:/.$-])herdr\s+[a-z]").unwrap();
        assert!(
            !herdr_command.is_match(skill),
            "old command in the skill: {:?}",
            herdr_command.find(skill).map(|found| found.as_str())
        );
        assert!(skill.contains("momo pane"));
    }
}
