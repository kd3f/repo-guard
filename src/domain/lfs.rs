//! Git LFS pointer detection and repair command generation.

/// Git LFS pointer file prefix per the specification.
pub const LFS_POINTER_PREFIX: &str = "version https://git-lfs.github.com/spec/v1";

/// Returns true when blob content is a Git LFS pointer.
pub fn is_lfs_pointer(content: &[u8]) -> bool {
    content.starts_with(LFS_POINTER_PREFIX.as_bytes())
}

/// Generate exact copy-paste repair commands for a large regular Git blob.
pub fn repair_commands(path: &str) -> Vec<String> {
    vec![
        format!("git lfs track --filename \"{path}\""),
        "git add .gitattributes".into(),
        format!("git add --renormalize -- \"{path}\""),
        "repo-guard check".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_lfs_pointer() {
        let content = b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 123\n";
        assert!(is_lfs_pointer(content));
    }

    #[test]
    fn repair_commands_include_exact_path() {
        let cmds = repair_commands("linceo/SEAF/Binari.zip");
        assert!(cmds[0].contains("Binari.zip"));
        assert!(cmds.iter().any(|c| c.contains("renormalize")));
    }
}
