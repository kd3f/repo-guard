//! Process execution abstractions for external commands.

use std::io;
use std::process::{Command, Output};

use thiserror::Error;

/// Result of running an external command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.status_code == 0
    }

    pub fn first_line_stdout(&self) -> Option<String> {
        self.stdout
            .lines()
            .next()
            .map(str::trim)
            .map(str::to_string)
    }
}

/// Narrow trait for spawning external processes (Git, Git LFS, etc.).
pub trait ProcessRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&str>) -> io::Result<ProcessOutput>;
}

/// Runs real system processes via `std::process::Command`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&str>) -> io::Result<ProcessOutput> {
        let mut command = Command::new(program);
        command.args(args);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }

        let output: Output = command.output()?;
        Ok(ProcessOutput {
            status_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[derive(Debug, Error)]
pub enum ProcessProbeError {
    #[error("failed to run {program}: {source}")]
    Io {
        program: String,
        #[source]
        source: io::Error,
    },
}

/// In-memory process runner for unit tests.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct FakeProcessRunner {
    pub responses: std::collections::HashMap<(String, Vec<String>), io::Result<ProcessOutput>>,
}

#[cfg(test)]
impl FakeProcessRunner {
    pub fn insert(
        mut self,
        program: &str,
        args: &[&str],
        result: io::Result<ProcessOutput>,
    ) -> Self {
        let key = (
            program.to_string(),
            args.iter().map(|s| (*s).to_string()).collect(),
        );
        self.responses.insert(key, result);
        self
    }
}

#[cfg(test)]
impl ProcessRunner for FakeProcessRunner {
    fn run(&self, program: &str, args: &[&str], _cwd: Option<&str>) -> io::Result<ProcessOutput> {
        let key = (
            program.to_string(),
            args.iter().map(|s| (*s).to_string()).collect(),
        );
        match self.responses.get(&key) {
            Some(Ok(output)) => Ok(output.clone()),
            Some(Err(err)) => Err(io::Error::new(err.kind(), err.to_string())),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no fake response for {program} {args:?}"),
            )),
        }
    }
}
