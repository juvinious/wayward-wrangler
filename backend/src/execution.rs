//! Linux durable-file and process-identity primitives for a single project.
//! No workflow transition or model policy lives here.
use crate::codec::{canonical, sha256};
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
pub fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub fn write(path: &Path, value: &Value, immutable: bool) -> Result<(), String> {
    let bytes = canonical(value).map_err(|e| e.to_string())?;
    write_bytes(path, &bytes, immutable)
}
pub fn write_bytes(path: &Path, bytes: &[u8], immutable: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("missing parent directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = parent.join(format!(
        ".publish-{}-{}",
        std::process::id(),
        random_token()?
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    let outcome = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if immutable {
            match fs::hard_link(&tmp, path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if fs::read(path).map_err(|e| e.to_string())? != bytes {
                        return Err(format!("Immutable content conflict: {}", path.display()));
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        } else {
            fs::rename(&tmp, path).map_err(|e| e.to_string())?;
        }
        File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| e.to_string())
    })();
    let _ = fs::remove_file(&tmp);
    outcome
}

pub struct FileLock(File);
impl FileLock {
    pub fn exclusive(path: &Path) -> Result<Self, String> {
        fs::create_dir_all(path.parent().ok_or("lock path has no parent")?)
            .map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| e.to_string())?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(format!("Already owned: {}", path.display()));
        }
        Ok(Self(file))
    }
}
impl Drop for FileLock {
    fn drop(&mut self) {
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

pub fn contained_operation(state: &Path, directory: &Path) -> Result<PathBuf, String> {
    let state = state.canonicalize().map_err(|e| e.to_string())?;
    if !directory.is_absolute() || !directory.starts_with(&state) {
        return Err("Operation path must be inside the selected state directory".into());
    }
    let parent = directory.parent().ok_or("Operation path has no parent")?;
    let mut existing = parent;
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or("Operation parent has no existing ancestor")?;
    }
    if !existing
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&state)
    {
        return Err("Operation parent leaves selected state directory".into());
    }
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if !parent
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&state)
    {
        return Err("Operation parent leaves selected state directory".into());
    }
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let directory = directory.canonicalize().map_err(|e| e.to_string())?;
    if !directory.starts_with(&state) {
        return Err("Operation path must be inside the selected state directory".into());
    }
    Ok(directory)
}

pub fn process_identity(pid: u32) -> Result<Value, String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|e| e.to_string())?;
    let after = stat
        .rfind(')')
        .ok_or("malformed process stat")?
        .checked_add(2)
        .ok_or("malformed process stat")?;
    let fields: Vec<&str> = stat[after..].split_whitespace().collect();
    if fields.len() <= 19 {
        return Err("malformed process stat".into());
    }
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
    Ok(serde_json::json!({"pid":pid,"start":fields[19],"boot":boot,"state":fields[0]}))
}
pub fn alive(expected: &Value) -> bool {
    let Some(pid) = expected["pid"].as_u64() else {
        return false;
    };
    let Ok(actual) = process_identity(pid as u32) else {
        return false;
    };
    actual["pid"] == expected["pid"]
        && actual["start"] == expected["start"]
        && actual["boot"] == expected["boot"]
        && actual["state"] != "Z"
}
pub fn identity_matches(a: &Value, b: &Value) -> bool {
    ["pid", "start", "boot"]
        .iter()
        .all(|key| a[*key] == b[*key])
}
pub fn signal_owned(expected: &Value, signal: libc::c_int) -> Result<(), String> {
    if !alive(expected) {
        return Err("Target process identity is no longer live".into());
    }
    let pid = expected["pid"].as_i64().ok_or("invalid target PID")? as libc::pid_t;
    // pidfd pins the specific process before checking identity and signalling.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as libc::c_int };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let outcome = (|| {
        if !alive(expected) {
            return Err("Target process identity changed before signal".into());
        }
        let status = unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd, signal, 0, 0) };
        if status != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    })();
    unsafe { libc::close(fd) };
    outcome
}
pub fn hash_file(path: &Path) -> Result<String, String> {
    Ok(sha256(&fs::read(path).map_err(|e| e.to_string())?))
}
