//! Working-tree scan logic.

use crate::domain::config::RepoGuardConfig;
use crate::domain::report::Issue;

const BYTES_PER_MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub path: String,
    pub size_bytes: u64,
    pub ignored: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanEvaluation {
    pub files: Vec<ScannedFile>,
    pub issues: Vec<Issue>,
}

impl ScanEvaluation {
    pub fn total_scanned(&self) -> usize {
        self.files.iter().filter(|f| !f.ignored).count()
    }

    pub fn ignored_count(&self) -> usize {
        self.files.iter().filter(|f| f.ignored).count()
    }

    pub fn large_files(&self, config: &RepoGuardConfig) -> Vec<&ScannedFile> {
        let threshold = config.lfs_threshold_mib * BYTES_PER_MIB;
        self.files
            .iter()
            .filter(|f| !f.ignored && f.size_bytes >= threshold)
            .collect()
    }

    pub fn largest_files(&self, limit: usize) -> Vec<&ScannedFile> {
        let mut files: Vec<_> = self.files.iter().filter(|f| !f.ignored).collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
        files.truncate(limit);
        files
    }
}

pub fn evaluate_scan(files: Vec<ScannedFile>, config: &RepoGuardConfig) -> ScanEvaluation {
    let mut evaluation = ScanEvaluation {
        files,
        issues: Vec::new(),
    };

    evaluation.issues.push(Issue::info(
        "scan.summary",
        format!(
            "Scanned {} files ({} ignored)",
            evaluation.total_scanned(),
            evaluation.ignored_count()
        ),
    ));

    let large_files: Vec<_> = evaluation
        .large_files(config)
        .into_iter()
        .cloned()
        .collect();
    for file in &large_files {
        evaluation.issues.push(Issue::warning(
            "scan.large_file",
            format!(
                "{} is {} bytes and likely requires Git LFS (threshold {} MiB)",
                file.path, file.size_bytes, config.lfs_threshold_mib
            ),
        ));
    }

    for folder in detect_generated_folders(&evaluation.files) {
        evaluation.issues.push(Issue::warning(
            "scan.generated_folder",
            format!("Potential generated or build output folder: {folder}"),
        ));
    }

    evaluation
}

fn detect_generated_folders(files: &[ScannedFile]) -> Vec<String> {
    let markers = [
        "target/",
        "build/",
        "dist/",
        "node_modules/",
        "bin/",
        "obj/",
        ".vs/",
    ];
    let mut found = Vec::new();
    for file in files {
        for marker in markers {
            if file.path.starts_with(marker) || file.path.contains(&format!("/{marker}")) {
                let folder = marker.trim_end_matches('/').to_string();
                if !found.contains(&folder) {
                    found.push(folder);
                }
            }
        }
    }
    found
}
