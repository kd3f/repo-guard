use repo_guard::application;
use repo_guard::cli::{
    Cli, Commands, HistoryCommands, HookCommands, IgnoreCommands, IntegrateCommands, LfsCommands,
    PresetCommands, RemoteCommands,
};
use repo_guard::domain;
use repo_guard::domain::integrate::IntegrationTarget;
use repo_guard::output::format::{render_report, OutputFormat};

use std::env;
use std::process;

use clap::Parser;

fn main() {
    let exit_code = match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("repo-guard: {err}");
            domain::exit_code::ExitCode::InternalError.as_i32()
        }
    };
    process::exit(exit_code);
}

fn run() -> Result<i32, String> {
    let cli = Cli::parse();
    let format: OutputFormat = cli.format.into();
    let cwd =
        env::current_dir().map_err(|err| format!("failed to read current directory: {err}"))?;

    let (report, exit_code) = match cli.command {
        Commands::Doctor => application::doctor::run_doctor_system(&cwd),
        Commands::Detect => application::detect::run_detect_system(&cwd),
        Commands::Init { apply, .. } => application::init::run_init_system(&cwd, apply),
        Commands::Install { apply, .. } => application::install::run_install_system(&cwd, apply),
        Commands::Check => application::check::run_check_system(&cwd),
        Commands::Scan => application::scan::run_scan_system(&cwd),
        Commands::Lfs { command } => match command {
            LfsCommands::Check => application::lfs_check::run_lfs_check_system(&cwd),
        },
        Commands::Ignore { command } => match command {
            IgnoreCommands::Check => application::ignore_check::run_ignore_check_system(&cwd),
        },
        Commands::Preset { command } => match command {
            PresetCommands::List => application::preset::run_preset_list(),
            PresetCommands::Apply { name, apply, .. } => {
                application::preset::run_preset_apply_system(&cwd, &name, apply)
            }
        },
        Commands::Remote { command } => match command {
            RemoteCommands::Check { require_origin } => {
                application::remote_check::run_remote_check_system(&cwd, require_origin)
            }
        },
        Commands::Hook { command } => match command {
            HookCommands::PreCommit => application::hook::run_hook_pre_commit_system(&cwd),
            HookCommands::PrePush => application::hook::run_hook_pre_push_stdio(&cwd),
        },
        Commands::History { command } => match command {
            HistoryCommands::Check {
                range,
                all,
                advisory,
            } => application::history_check::run_history_check_system(
                &cwd,
                application::history_check::HistoryCheckRequest {
                    range,
                    all,
                    advisory,
                },
            ),
        },
        Commands::Integrate { command } => match command {
            IntegrateCommands::Detect => application::integrate::run_integrate_system(
                &cwd,
                application::integrate::IntegrateRequest::Detect,
            ),
            IntegrateCommands::PreCommit { .. } => application::integrate::run_integrate_system(
                &cwd,
                application::integrate::IntegrateRequest::Print(IntegrationTarget::PreCommit),
            ),
            IntegrateCommands::Husky { .. } => application::integrate::run_integrate_system(
                &cwd,
                application::integrate::IntegrateRequest::Print(IntegrationTarget::Husky),
            ),
            IntegrateCommands::Lefthook { .. } => application::integrate::run_integrate_system(
                &cwd,
                application::integrate::IntegrateRequest::Print(IntegrationTarget::Lefthook),
            ),
            IntegrateCommands::Ci { .. } => application::integrate::run_integrate_system(
                &cwd,
                application::integrate::IntegrateRequest::Print(IntegrationTarget::Ci),
            ),
        },
    };

    let rendered =
        render_report(&report, format).map_err(|err| format!("failed to render report: {err}"))?;
    println!("{rendered}");
    Ok(exit_code.as_i32())
}

#[cfg(test)]
mod tests {
    use super::*;
    use repo_guard::domain::exit_code::ExitCode;
    use repo_guard::domain::report::CommandReport;

    #[test]
    fn doctor_command_returns_expected_exit_code() {
        let cwd = env::current_dir().expect("cwd");
        let (_, exit_code) = application::doctor::run_doctor_system(&cwd);
        assert!(exit_code.as_i32() >= 0);
    }

    #[test]
    fn render_doctor_report_as_json() {
        let report = CommandReport::new("doctor", ExitCode::Success).finalize(ExitCode::Success);
        let json = render_report(&report, OutputFormat::Json).unwrap();
        assert!(json.contains("\"schema_version\""));
    }
}
