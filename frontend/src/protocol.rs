use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub const FRAME_MAX: usize = 1024 * 1024;
pub const COMMAND_MAX: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub v: u8,
    pub kind: String,
    pub request_id: String,
    pub body: Value,
}

impl Frame {
    pub fn new(kind: &str, request_id: &str, body: Value) -> Self {
        Self { v: 1, kind: kind.into(), request_id: request_id.into(), body }
    }
}

pub fn read_frame(stream: &mut UnixStream) -> io::Result<Frame> {
    let mut bytes = Vec::new();
    let mut octet = [0u8];
    loop {
        if stream.read(&mut octet)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
        }
        if octet[0] == b'\n' { break; }
        if bytes.len() == FRAME_MAX {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
        }
        bytes.push(octet[0]);
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if text.contains('\n') || text.contains('\r') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unescaped newline"));
    }
    let frame: Frame = serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if frame.v != 1 || frame.request_id.is_empty() || !frame.body.is_object() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid envelope"));
    }
    Ok(frame)
}

pub fn write_frame(stream: &mut UnixStream, frame: &Frame) -> io::Result<()> {
    let bytes = serde_json::to_vec(frame).map_err(io::Error::other)?;
    if bytes.len() > FRAME_MAX || (frame.kind == "command" && bytes.len() > COMMAND_MAX) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame too large"));
    }
    stream.write_all(&bytes)?;
    stream.write_all(b"\n")
}

pub fn connect(socket: &str, project_key: &str, release: &str, channel: &str) -> io::Result<UnixStream> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let id = format!("{}-{}", std::process::id(), channel);
    write_frame(&mut stream, &Frame::new("hello", &id, serde_json::json!({
        "protocol": {"major": 1, "minor": 10}, "release_id": release,
        "project_key": project_key, "client_id": format!("cli-{}", std::process::id()), "channel": channel
    })))?;
    let reply = read_frame(&mut stream)?;
    if reply.kind != "hello_result" || reply.request_id != id || reply.body.get("accepted") != Some(&Value::Bool(true)) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("handshake refused: {:?}", reply.body)));
    }
    Ok(stream)
}

pub fn connect_view(socket:&str,project_key:&str,release:&str,id:&str,token:&str)->io::Result<UnixStream>{
    let mut stream=UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request=format!("view-{}",std::process::id());
    write_frame(&mut stream,&Frame::new("hello",&request,serde_json::json!({
        "protocol":{"major":1,"minor":1},"release_id":release,"project_key":project_key,
        "client_id":request,"channel":"view","view_id":id,"view_token":token
    })))?;
    let reply=read_frame(&mut stream)?;
    if reply.kind!="hello_result" || reply.request_id!=request || reply.body["accepted"]!=true{
        return Err(io::Error::new(io::ErrorKind::PermissionDenied,"read-only view grant refused"));
    }
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    #[test]
    fn frame_roundtrip_and_ceiling() {
        let (mut a, mut b) = UnixStream::pair().unwrap();
        let sent = Frame::new("hello", "1", serde_json::json!({}));
        write_frame(&mut a, &sent).unwrap();
        assert_eq!(read_frame(&mut b).unwrap().request_id, "1");
        let (mut a, mut b) = UnixStream::pair().unwrap();
        std::thread::spawn(move || { let _ = a.write_all(&vec![b'x'; FRAME_MAX + 1]); });
        assert!(read_frame(&mut b).unwrap_err().to_string().contains("frame too large"));
    }
}
