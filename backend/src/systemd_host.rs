//! Project-scoped process hosting. Systemd corroborates a recorded attempt;
//! application operation files remain the authority for dispatch and completion.
use std::{path::Path, time::Duration};
use zbus::{Connection, OwnedObjectPath, Proxy, Variant};

const DEST: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";

#[derive(Debug, Clone)]
pub struct HostStatus {
    pub name: String,
    pub active: String,
    pub sub: String,
    pub invocation_id: String,
    pub control_group: String,
    pub main_pid: u32,
    pub result: String,
    pub restart: String,
    pub exit_type: String,
    pub remain_after_exit: bool,
    pub collect_mode: String,
}

pub struct UserManager {
    connection: Connection,
}
fn valid_unit(name: &str) -> bool {
    let Some(rest) = name
        .strip_prefix("waywr-op-")
        .and_then(|value| value.strip_suffix(".service"))
    else {
        return false;
    };
    let fields = rest.split('-').collect::<Vec<_>>();
    fields.len() == 3
        && [32, 16, 32].into_iter().zip(fields).all(|(len, value)| {
            value.len() == len
                && value
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
}
impl UserManager {
    pub async fn connect() -> Result<Self, String> {
        let connection = tokio::time::timeout(Duration::from_secs(2), Connection::session())
            .await
            .map_err(|_| "User bus connection timed out".to_owned())?
            .map_err(|e| format!("User bus unavailable: {e}"))?;
        Ok(Self { connection })
    }
    async fn manager(&self) -> Result<Proxy<'_>, String> {
        Proxy::new(&self.connection, DEST, MANAGER_PATH, MANAGER)
            .await
            .map_err(|e| e.to_string())
    }
    pub async fn start(
        &self,
        name: &str,
        binary: &Path,
        args: &[String],
        cwd: &Path,
    ) -> Result<(), String> {
        if !valid_unit(name) {
            return Err("Invalid operation unit name".into());
        }
        let binary = binary.canonicalize().map_err(|e| e.to_string())?;
        if !binary.is_file() || !binary.is_absolute() || !cwd.is_absolute() || !cwd.is_dir() {
            return Err("Operation executable or directory is unavailable".into());
        }
        let executable = binary.to_str().ok_or("Non-UTF8 runner executable")?;
        let directory = cwd.to_str().ok_or("Non-UTF8 operation directory")?;
        let argv = std::iter::once(executable.to_owned())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>();
        let command = vec![(executable.to_owned(), argv, false)];
        let props: Vec<(&str, Variant<'_>)> = vec![
            (
                "Description",
                Variant::new(format!("Wayward Wrangler operation {name}")),
            ),
            ("ExecStart", Variant::new(command)),
            ("WorkingDirectory", Variant::new(directory.to_owned())),
            ("Type", Variant::new("exec")),
            ("ExitType", Variant::new("cgroup")),
            ("Restart", Variant::new("no")),
            ("RemainAfterExit", Variant::new(true)),
            ("CollectMode", Variant::new("inactive")),
            ("KillMode", Variant::new("control-group")),
            ("TimeoutStartUSec", Variant::new(10_000_000u64)),
            ("TimeoutStopUSec", Variant::new(3_000_000u64)),
            ("SendSIGKILL", Variant::new(true)),
            ("OOMPolicy", Variant::new("continue")),
            ("StandardInput", Variant::new("null")),
            ("StandardOutput", Variant::new("null")),
            ("StandardError", Variant::new("null")),
        ];
        let manager = self.manager().await?;
        let _: OwnedObjectPath = tokio::time::timeout(
            Duration::from_secs(10),
            manager.call(
                "StartTransientUnit",
                &(
                    name,
                    "fail",
                    props,
                    Vec::<(&str, Vec<(&str, Variant<'_>)>)>::new(),
                ),
            ),
        )
        .await
        .map_err(|_| "Start request outcome unknown: timeout after send".to_owned())?
        .map_err(|e| format!("Start request outcome requires reconciliation: {e}"))?;
        Ok(())
    }
    pub async fn status(&self, name: &str) -> Result<HostStatus, String> {
        if !valid_unit(name) {
            return Err("Invalid operation unit name".into());
        }
        tokio::time::timeout(Duration::from_secs(2), self.status_inner(name))
            .await
            .map_err(|_| {
                "Exact unit status timed out; outcome requires reconciliation".to_owned()
            })?
    }
    async fn status_inner(&self, name: &str) -> Result<HostStatus, String> {
        let manager = self.manager().await?;
        let path: OwnedObjectPath = manager
            .call("GetUnit", &(name,))
            .await
            .map_err(|e| format!("Cannot resolve exact operation unit {name}: {e}"))?;
        let unit = Proxy::new(
            &self.connection,
            DEST,
            path.as_str(),
            "org.freedesktop.systemd1.Unit",
        )
        .await
        .map_err(|e| e.to_string())?;
        let service = Proxy::new(
            &self.connection,
            DEST,
            path.as_str(),
            "org.freedesktop.systemd1.Service",
        )
        .await
        .map_err(|e| e.to_string())?;
        let id: String = unit.get_property("Id").await.map_err(|e| e.to_string())?;
        if id != name {
            return Err("Resolved systemd unit identity mismatch".into());
        }
        let invocation: Vec<u8> = unit
            .get_property("InvocationID")
            .await
            .map_err(|e| format!("InvocationID: {e}"))?;
        if invocation.len() != 16 {
            return Err("Invalid systemd InvocationID length".into());
        }
        Ok(HostStatus {
            name: id,
            active: unit
                .get_property("ActiveState")
                .await
                .map_err(|e| e.to_string())?,
            sub: unit
                .get_property("SubState")
                .await
                .map_err(|e| e.to_string())?,
            invocation_id: invocation
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            control_group: service
                .get_property("ControlGroup")
                .await
                .map_err(|e| e.to_string())?,
            main_pid: service
                .get_property("MainPID")
                .await
                .map_err(|e| e.to_string())?,
            result: service
                .get_property("Result")
                .await
                .map_err(|e| e.to_string())?,
            restart: service
                .get_property("Restart")
                .await
                .map_err(|e| e.to_string())?,
            exit_type: service
                .get_property("ExitType")
                .await
                .map_err(|e| e.to_string())?,
            remain_after_exit: service
                .get_property("RemainAfterExit")
                .await
                .map_err(|e| e.to_string())?,
            collect_mode: unit
                .get_property("CollectMode")
                .await
                .map_err(|e| e.to_string())?,
        })
    }
    pub async fn stop(&self, name: &str) -> Result<(), String> {
        if !valid_unit(name) {
            return Err("Invalid operation unit name".into());
        }
        let manager = self.manager().await?;
        let _: OwnedObjectPath = tokio::time::timeout(
            Duration::from_secs(2),
            manager.call("StopUnit", &(name, "fail")),
        )
        .await
        .map_err(|_| "Stop request outcome unknown; inspect exact unit".to_owned())?
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct OwnedTestUnit {
        name: String,
        armed: bool,
    }
    impl Drop for OwnedTestUnit {
        fn drop(&mut self) {
            if self.armed {
                let _ = std::process::Command::new("systemctl")
                    .args(["--user", "stop", &self.name])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
        }
    }
    #[tokio::test]
    #[ignore = "starts one unique test-owned systemd user unit; run only in isolated host acceptance"]
    async fn exact_start_and_reconciliation() {
        let name = format!(
            "waywr-op-{}-{}-{}.service",
            "a".repeat(32),
            "b".repeat(16),
            crate::execution::random_token().unwrap()
        );
        let mut owned = OwnedTestUnit {
            name: name.clone(),
            armed: true,
        };
        let manager = UserManager::connect().await.unwrap();
        manager
            .start(
                &name,
                Path::new("/usr/bin/sleep"),
                &["30".to_owned()],
                Path::new("/tmp"),
            )
            .await
            .unwrap();
        // Deliberately discard the start reply. Observation of the exact name
        // must recover identity; a second start is not a valid retry.
        let status = manager.status(&name).await.unwrap();
        assert_eq!(status.name, name);
        assert_eq!(status.exit_type, "cgroup");
        assert_eq!(status.restart, "no");
        assert!(status.remain_after_exit);
        assert!(!status.invocation_id.is_empty());
        let second = manager
            .start(
                &name,
                Path::new("/usr/bin/sleep"),
                &["30".to_owned()],
                Path::new("/tmp"),
            )
            .await;
        assert!(second.is_err());
        assert_eq!(
            manager.status(&name).await.unwrap().invocation_id,
            status.invocation_id
        );
        manager.stop(&name).await.unwrap();
        owned.armed = false;
    }
}
