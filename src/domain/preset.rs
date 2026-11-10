//! Preset management for opt-in `.gitignore` blocks.

use crate::domain::report::Issue;

pub const BLOCK_BEGIN: &str = "# repo-guard:begin preset=";
pub const BLOCK_END: &str = "# repo-guard:end";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetApplyPlan {
    pub preset: String,
    pub would_create: bool,
    pub would_update: bool,
    pub content: String,
}

pub fn managed_block(preset: &str, body: &str) -> String {
    format!("{BLOCK_BEGIN}{preset}\n{body}\n{BLOCK_END}\n")
}

pub fn list_builtin_presets() -> Vec<PresetInfo> {
    vec![
        PresetInfo {
            name: "python".into(),
            description: "Python bytecode, virtual environments".into(),
        },
        PresetInfo {
            name: "nodejs".into(),
            description: "Node.js dependencies and build output".into(),
        },
        PresetInfo {
            name: "dotnet".into(),
            description: "Visual Studio and .NET build artifacts".into(),
        },
        PresetInfo {
            name: "java".into(),
            description: "Java and Gradle build output".into(),
        },
        PresetInfo {
            name: "unity".into(),
            description: "Unity generated library and temp folders".into(),
        },
        PresetInfo {
            name: "unreal".into(),
            description: "Unreal Engine intermediate and saved folders".into(),
        },
        PresetInfo {
            name: "cmake".into(),
            description: "CMake build directories".into(),
        },
        PresetInfo {
            name: "jetbrains".into(),
            description: "JetBrains IDE project files".into(),
        },
        PresetInfo {
            name: "logs".into(),
            description: "Common logs and temporary files".into(),
        },
    ]
}

pub fn plan_preset_apply(
    preset: &str,
    preset_body: &str,
    existing_gitignore: Option<&str>,
) -> Result<PresetApplyPlan, String> {
    if list_builtin_presets().iter().all(|p| p.name != preset) {
        return Err(format!("unknown preset \"{preset}\""));
    }

    let block = managed_block(preset, preset_body);
    let (would_create, would_update, content) = match existing_gitignore {
        None | Some("") => (true, false, block),
        Some(existing) => {
            if existing.contains(&format!("{BLOCK_BEGIN}{preset}")) {
                let updated = replace_managed_block(existing, preset, preset_body);
                (false, updated != existing, updated)
            } else {
                let mut content = existing.to_string();
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&block);
                (false, true, content)
            }
        }
    };

    Ok(PresetApplyPlan {
        preset: preset.into(),
        would_create,
        would_update,
        content,
    })
}

fn replace_managed_block(existing: &str, preset: &str, body: &str) -> String {
    let begin = format!("{BLOCK_BEGIN}{preset}");
    let mut output = String::new();
    let mut in_block = false;
    for line in existing.lines() {
        if line.starts_with(&begin) {
            in_block = true;
            output.push_str(&managed_block(preset, body));
            continue;
        }
        if in_block {
            if line.starts_with(BLOCK_END) {
                in_block = false;
            }
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

pub fn preset_apply_issues(plan: &PresetApplyPlan, apply: bool) -> Vec<Issue> {
    let mut issues = Vec::new();
    if plan.would_create {
        issues.push(Issue::info(
            "preset.create_gitignore",
            "Would create .gitignore with managed preset block",
        ));
    } else if plan.would_update {
        issues.push(Issue::info(
            "preset.update_gitignore",
            format!("Would update managed preset block for {}", plan.preset),
        ));
    } else {
        issues.push(Issue::info(
            "preset.noop",
            format!("Preset {} is already applied", plan.preset),
        ));
    }
    if !apply {
        issues.push(Issue::info(
            "preset.dry_run",
            "Dry run only; pass --apply to modify .gitignore",
        ));
    }
    issues
}
