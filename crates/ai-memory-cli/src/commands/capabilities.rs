//! `ai-memory capabilities`: the optional launcher features this binary
//! supports. An orchestrator reads it once to decide whether to warn that a
//! worker's injection level or role will be ignored; it sets the variables
//! either way, since an older binary simply ignores them.

use crate::cli::CapabilitiesArgs;
use crate::commands::hook::InjectLevel;

/// Names this build apart from upstream's, which keeps the same version
/// number.
const BUILD: &str = "ownercode";
/// Where this build's source lives.
const REPOSITORY: &str = "https://github.com/NetoMouraJs/ai-memory";

/// The report, as one JSON object.
#[must_use]
pub fn report() -> serde_json::Value {
    let roles: Vec<&str> = ai_memory_core::SessionOrigin::ALL
        .iter()
        .map(|origin| origin.as_str())
        .collect();
    serde_json::json!({
        "build": BUILD,
        "repository": REPOSITORY,
        "hook_inject_env": InjectLevel::TOKENS,
        "session_role_env": roles,
        "session_label_env": true,
    })
}

/// Print the report.
///
/// # Errors
///
/// Returns an error only when stdout cannot be written.
pub fn run(args: &CapabilitiesArgs) -> anyhow::Result<()> {
    use std::io::Write as _;
    let report = report();
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{report}")?;
    } else {
        writeln!(stdout, "build: {BUILD}")?;
        writeln!(stdout, "repository: {REPOSITORY}")?;
        writeln!(
            stdout,
            "hook_inject_env: {}",
            InjectLevel::TOKENS.join(", ")
        )?;
        let roles: Vec<&str> = ai_memory_core::SessionOrigin::ALL
            .iter()
            .map(|origin| origin.as_str())
            .collect();
        writeln!(stdout, "session_role_env: {}", roles.join(", "))?;
        writeln!(stdout, "session_label_env: true")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::report;

    #[test]
    fn the_report_names_the_build_and_every_level_role_and_the_label() {
        assert_eq!(
            report(),
            serde_json::json!({
                "build": "ownercode",
                "repository": "https://github.com/NetoMouraJs/ai-memory",
                "hook_inject_env": ["all", "no-handoff", "none"],
                "session_role_env": ["automation", "builder", "reviewer", "qa"],
                "session_label_env": true,
            })
        );
    }
}
