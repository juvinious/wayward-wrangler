//! Supported Herdr Unix-socket control boundary. Herdr owns visible terminals;
//! durable runner files, not pane text, prove execution completion.
use crate::execution::{process_identity, random_token};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::{Path, PathBuf},
    time::Duration,
    thread,
};

pub struct Herdr {
    pub socket: PathBuf,
}

#[derive(Debug)]
pub enum CreateError {
    /// The request that can start a worker was never sent to Herdr.
    PreSend(String),
    /// layout.apply was attempted; its outcome requires reconciliation.
    MaybeSent(String),
}
impl std::fmt::Display for CreateError {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        match self {Self::PreSend(detail)|Self::MaybeSent(detail)=>f.write_str(detail)}
    }
}
impl Herdr {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
    fn connect(&self, seconds: u64) -> Result<UnixStream, String> {
        let stream = UnixStream::connect(&self.socket).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(seconds)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(seconds)))
            .map_err(|e| e.to_string())?;
        Ok(stream)
    }
    pub fn server_identity(&self) -> Result<Value, String> {
        let stream = self.connect(5)?;
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&raw mut peer).cast(),
                &raw mut length,
            )
        } != 0
            || length as usize != std::mem::size_of::<libc::ucred>()
        {
            return Err("Cannot verify Herdr socket peer".into());
        }
        if peer.uid != unsafe { libc::getuid() } {
            return Err("Herdr socket is not owned by this user".into());
        }
        process_identity(peer.pid as u32)
    }
    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut stream = self.connect(8)?;
        let request = json!({"id":random_token()?,"method":method,"params":params});
        let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        stream.write_all(&bytes).map_err(|e| e.to_string())?;
        let mut response = Vec::new();
        let mut block = [0u8; 4096];
        while response.len() <= 4 * 1024 * 1024 {
            let n = stream.read(&mut block).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            response.extend_from_slice(&block[..n]);
            if let Some(index) = response.iter().position(|byte| *byte == b'\n') {
                response.truncate(index);
                break;
            }
        }
        if response.len() > 4 * 1024 * 1024 {
            return Err("Herdr response exceeds 4 MiB".into());
        }
        let value: Value = serde_json::from_slice(&response).map_err(|e| e.to_string())?;
        if let Some(error) = value.get("error") {
            return Err(format!("Herdr {method}: {error}"));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| format!("Herdr {method}: missing result"))
    }
    pub fn workspace(&self, id: &str) -> Result<Value, String> {
        self.call("workspace.get", json!({"workspace_id":id}))
    }
    pub fn create(
        &self,
        workspace: &str,
        label: &str,
        command: &[String],
        cwd: &Path,
        initial_tab: Option<&str>,
    ) -> Result<Value, CreateError> {
        let mut params=json!({"workspace_id":workspace,"tab_label":label,"focus":false,
            "root":{"type":"pane","label":label,"cwd":cwd,"command":command}});
        if let Some(tab)=initial_tab{
            match self.call("tab.get",json!({"tab_id":tab})){
                Ok(found)=>{
                    if found["tab"]["workspace_id"]!=workspace{return Err(CreateError::PreSend("Bootstrap tab belongs to another workspace".into()));}
                    let layout=self.call("layout.export",json!({"tab_id":tab})).map_err(CreateError::PreSend)?;
                    let root=&layout["layout"]["root"];
                    if root["type"]!="pane" || root.get("command").is_some(){
                        return Err(CreateError::PreSend("Bootstrap tab is no longer an idle project shell; inspect before dispatch".into()));
                    }
                    let pane=root["pane_id"].as_str().ok_or_else(||CreateError::PreSend("Bootstrap tab lacks pane".into()))?;
                    // A freshly created Fish tab can report startup work before
                    // its prompt becomes idle. Never replace a busy tab; wait a
                    // bounded interval and recheck immediately before apply.
                    let mut idle=false;
                    for attempt in 0..21 {
                        let process=self.call("pane.process_info",json!({"pane_id":pane})).map_err(CreateError::PreSend)?;
                        let info=&process["process_info"];
                        let shell=info["shell_pid"].as_u64().ok_or_else(||CreateError::PreSend("Bootstrap tab lacks shell PID".into()))?;
                        if info["foreground_processes"].as_array().is_some_and(|items|
                            items.len()==1 && items[0]["pid"]==shell){idle=true;break;}
                        if attempt<20 {thread::sleep(Duration::from_millis(100));}
                    }
                    if !idle {
                        return Err(CreateError::PreSend("Bootstrap tab contains foreground work after 2 seconds; no worker launch was sent".into()));
                    }
                    params.as_object_mut().unwrap().remove("workspace_id");
                    params["tab_id"]=tab.into();
                }
                Err(error) if error.contains("tab_not_found")=>{},
                Err(error)=>return Err(CreateError::PreSend(error)),
            }
        }
        let value = self.call("layout.apply",params).map_err(CreateError::MaybeSent)?;
        value
            .get("layout")
            .cloned()
            .ok_or_else(||CreateError::MaybeSent("Herdr layout.apply omitted layout".into()))
    }
    pub fn find(&self, token: &str, runner_binary: &Path) -> Result<Vec<Value>, String> {
        fn visit(node: &Value, token: &str, runner: &str, layout: &Value, out: &mut Vec<Value>) {
            if node["type"] == "pane" {
                let args = node["command"].as_array();
                if args.is_some_and(|args| {
                    args.iter().any(|v| v == token)
                        && args.first().and_then(Value::as_str) == Some(runner)
                }) {
                    out.push(json!({"pane_id":node["pane_id"],"tab_id":layout["tab_id"],
                        "workspace_id":layout["workspace_id"]}));
                }
            } else if node["type"] != "pane" {
                visit(&node["first"], token, runner, layout, out);
                visit(&node["second"], token, runner, layout, out);
            }
        }
        let runner = runner_binary.to_string_lossy();
        // A tab may exit between tab.list and layout.export. Restart the whole
        // scan so a concurrent tab removal cannot hide a matching owned pane.
        for attempt in 0..4 {
            let tabs = self.call("tab.list", json!({}))?;
            let mut found = Vec::new();
            let mut changed=false;
            for tab in tabs["tabs"].as_array().ok_or("Herdr tab.list omitted tabs")? {
                match self.call("layout.export",json!({"tab_id":tab["tab_id"]})) {
                    Ok(layout)=>{
                        let layout=&layout["layout"];
                        visit(&layout["root"], token, &runner, layout, &mut found);
                    }
                    Err(error) if error.contains("layout_not_found")=>{changed=true;break;}
                    Err(error)=>return Err(error),
                }
            }
            if !changed{return Ok(found);}
            if attempt<3{thread::sleep(Duration::from_millis(25));}
        }
        Err("Herdr tabs changed during four identity scans; inspect before retrying".into())
    }
    pub fn pane(&self, id: &str) -> Result<Value, String> {
        let value = self.call("pane.get", json!({"pane_id":id}))?;
        value
            .get("pane")
            .cloned()
            .ok_or("Herdr pane.get omitted pane".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{BufRead,BufReader},os::unix::net::UnixListener};
    #[test]
    fn create_waits_for_idle_bootstrap_before_sending_layout() {
        let root=tempfile::tempdir().unwrap();
        let socket=root.path().join("herdr.sock");
        let listener=UnixListener::bind(&socket).unwrap();
        let responses=[
            ("tab.get",json!({"tab":{"workspace_id":"w1"}})),
            ("layout.export",json!({"layout":{"root":{"type":"pane","pane_id":"w1:p1"}}})),
            ("pane.process_info",json!({"process_info":{"shell_pid":42,"foreground_processes":[{"pid":43}]}})),
            ("pane.process_info",json!({"process_info":{"shell_pid":42,"foreground_processes":[{"pid":42}]}})),
            ("layout.apply",json!({"layout":{"tab_id":"w1:t1","workspace_id":"w1"}})),
        ];
        let server=thread::spawn(move||{
            for (method,result) in responses {
                let (mut peer,_)=listener.accept().unwrap();
                let mut request=String::new();
                BufReader::new(peer.try_clone().unwrap()).read_line(&mut request).unwrap();
                let request:Value=serde_json::from_str(&request).unwrap();
                assert_eq!(request["method"],method);
                peer.write_all(serde_json::to_string(&json!({"result":result})).unwrap().as_bytes()).unwrap();
                peer.write_all(b"\n").unwrap();
            }
        });
        let created=Herdr::new(&socket).create("w1","fake worker",&["/bin/true".into()],root.path(),Some("w1:t1")).unwrap();
        assert_eq!(created["tab_id"],"w1:t1");
        server.join().unwrap();
    }
    #[test]
    fn find_restarts_scan_when_listed_tab_disappears() {
        let root=tempfile::tempdir().unwrap();
        let socket=root.path().join("herdr.sock");
        let listener=UnixListener::bind(&socket).unwrap();
        let replies=[
            json!({"result":{"tabs":[{"tab_id":"w1:t1"}]}}),
            json!({"error":{"code":"layout_not_found","message":"layout target not found"}}),
            json!({"result":{"tabs":[{"tab_id":"w2:t1"}]}}),
            json!({"result":{"layout":{"workspace_id":"w2","tab_id":"w2:t1",
                "root":{"type":"pane","pane_id":"w2:p1",
                    "command":["/bin/waywr","--ui-token","token"]}}}}),
        ];
        let server=thread::spawn(move||{
            for reply in replies {
                let (mut peer,_)=listener.accept().unwrap();
                let mut request=String::new();
                BufReader::new(peer.try_clone().unwrap()).read_line(&mut request).unwrap();
                assert!(!request.is_empty());
                peer.write_all(serde_json::to_string(&reply).unwrap().as_bytes()).unwrap();
                peer.write_all(b"\n").unwrap();
            }
        });
        let found=Herdr::new(&socket).find("token",Path::new("/bin/waywr")).unwrap();
        assert_eq!(found,vec![json!({"workspace_id":"w2","tab_id":"w2:t1","pane_id":"w2:p1"})]);
        server.join().unwrap();
    }
}
