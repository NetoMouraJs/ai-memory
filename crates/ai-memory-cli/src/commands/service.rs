//! `ai-memory service`: start, stop and inspect the Windows background host
//! that `setup` registers under the per-user `Run` key, without waiting for
//! the next logon or killing its processes by pid.
//!
//! `stop` is the host's own cooperative stop ([`service_host::stop`]); the
//! logon entry stays, so the next logon starts the host again. `start` runs
//! the host's detached launcher, which places the supervisor outside the
//! caller's job, so a host started from an app's terminal outlives the app.

use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};

use super::service_host::{self, HostState};
use super::user_service::SystemRunner;
use crate::cli::{ServiceArgs, ServiceCommand, ServiceHostArgs, ServiceStatusArgs};
use crate::config::Config;
use crate::http_client::{ServerEndpoint, ServerProbe, probe_server};

/// How long `start` waits for the new server to answer.
const START_TIMEOUT: Duration = Duration::from_secs(30);

/// What `start` does, given whether the host runs and whether something
/// answers on the configured server URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartPlan {
    /// The host already runs; nothing to do.
    AlreadyRunning,
    /// Another server (not the host) holds the URL; a host started now would
    /// fail on the port and give up.
    Occupied,
    Launch,
}

fn plan_start(host_running: bool, responding: bool) -> StartPlan {
    if host_running {
        return StartPlan::AlreadyRunning;
    }
    if responding {
        return StartPlan::Occupied;
    }
    StartPlan::Launch
}

/// The host's state, as `status --json` prints it.
fn status_report(host: Option<HostState>, responding: bool, url: &str) -> serde_json::Value {
    serde_json::json!({
        "running": host.is_some(),
        "supervisor_pid": host.map(|state| state.supervisor_pid),
        "server_pid": host.and_then(|state| state.server_pid),
        "responding": responding,
        "server_url": url,
    })
}

/// Run `service`.
///
/// # Errors
/// On a platform other than Windows; when the host cannot be started or
/// stopped; when another server already holds the configured URL; or when a
/// started host's server does not answer within [`START_TIMEOUT`].
pub async fn run(config: &Config, args: ServiceArgs, config_path: Option<&Path>) -> Result<()> {
    if !cfg!(windows) {
        bail!(
            "`ai-memory service` controls the Windows logon host; on this platform use \
             `systemctl --user` or `launchctl` for the unit `ai-memory setup` installed"
        );
    }
    let state_path = service_host::state_file(&config.data_dir);
    let endpoint = ServerEndpoint::from_config_resolving_auth(config).await;
    match args.command {
        ServiceCommand::Start => {
            let host = service_host::running(&state_path);
            let responding = is_reachable(&endpoint).await;
            match plan_start(host.is_some(), responding) {
                StartPlan::AlreadyRunning => {
                    println!("ai-memory service already running at {}", endpoint.url);
                }
                StartPlan::Occupied => bail!(
                    "another ai-memory server already answers at {} and is not the service; \
                     stop it first",
                    endpoint.url
                ),
                StartPlan::Launch => {
                    service_host::run(config, ServiceHostArgs { supervise: false }, config_path)
                        .await?;
                    if !super::setup::wait_until_reachable(&endpoint, START_TIMEOUT).await {
                        bail!(
                            "the service started but its server did not answer at {} within {}s; \
                             see {}",
                            endpoint.url,
                            START_TIMEOUT.as_secs(),
                            config.data_dir.join("logs").display()
                        );
                    }
                    println!("ai-memory service started at {}", endpoint.url);
                }
            }
        }
        ServiceCommand::Stop => {
            let was_running = service_host::running(&state_path).is_some();
            service_host::stop(&state_path, &SystemRunner)?;
            if was_running {
                println!("ai-memory service stopped");
            } else {
                println!("ai-memory service was not running");
            }
        }
        ServiceCommand::Status(ServiceStatusArgs { json }) => {
            let host = service_host::running(&state_path);
            let responding = is_reachable(&endpoint).await;
            let report = status_report(host, responding, &endpoint.url);
            if json {
                println!("{report}");
            } else {
                print_status(host, responding, &endpoint.url);
            }
        }
    }
    Ok(())
}

async fn is_reachable(endpoint: &ServerEndpoint) -> bool {
    matches!(probe_server(endpoint).await, ServerProbe::Reachable)
}

fn print_status(host: Option<HostState>, responding: bool, url: &str) {
    match host {
        Some(state) => {
            let server = state
                .server_pid
                .map_or_else(|| "starting".to_owned(), |pid| pid.to_string());
            println!(
                "ai-memory service: running (supervisor {}, server {server})",
                state.supervisor_pid
            );
        }
        None => println!("ai-memory service: stopped"),
    }
    let answer = if responding {
        "answers"
    } else {
        "does not answer"
    };
    println!("server at {url}: {answer}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A running host is left alone, a URL another server holds is refused,
    /// and only a free URL starts the host.
    #[test]
    fn start_launches_only_when_no_host_runs_and_the_url_is_free() {
        assert_eq!(plan_start(true, true), StartPlan::AlreadyRunning);
        assert_eq!(plan_start(true, false), StartPlan::AlreadyRunning);
        assert_eq!(plan_start(false, true), StartPlan::Occupied);
        assert_eq!(plan_start(false, false), StartPlan::Launch);
    }

    #[test]
    fn status_reports_the_host_pids_and_whether_the_server_answers() {
        let url = "http://127.0.0.1:49374/";
        assert_eq!(
            status_report(
                Some(HostState {
                    supervisor_pid: 10,
                    server_pid: Some(20),
                }),
                true,
                url,
            ),
            serde_json::json!({
                "running": true,
                "supervisor_pid": 10,
                "server_pid": 20,
                "responding": true,
                "server_url": url,
            })
        );
        assert_eq!(
            status_report(None, false, url),
            serde_json::json!({
                "running": false,
                "supervisor_pid": null,
                "server_pid": null,
                "responding": false,
                "server_url": url,
            })
        );
    }
}
