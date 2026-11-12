use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub use waywr_backend::ipc_limits::{RESPONSE_FRAME_MAX as FRAME_MAX,COMMAND_MAX};
use waywr_backend::ipc_limits::REQUEST_FRAME_MAX;

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
    use std::os::fd::AsRawFd;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        // Peek for the delimiter before consuming a chunk. A temporary BufReader
        // would discard a following coalesced frame when this function returns.
        // The stream has one reader; never consume beyond this reply's newline.
        let available = unsafe {
            libc::recv(stream.as_raw_fd(), chunk.as_mut_ptr().cast(), chunk.len(), libc::MSG_PEEK)
        };
        if available < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted { continue; }
            return Err(error);
        }
        if available == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
        }
        let available = available as usize;
        let end = chunk[..available].iter().position(|byte| *byte == b'\n');
        let content = end.unwrap_or(available);
        if content > FRAME_MAX - bytes.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
        }
        let consume = content + usize::from(end.is_some());
        stream.read_exact(&mut chunk[..consume])?;
        bytes.extend_from_slice(&chunk[..content]);
        if end.is_some() { break; }
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
    if bytes.len() > REQUEST_FRAME_MAX || (frame.kind == "command" && bytes.len() > COMMAND_MAX) {
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
    fn populated_snapshot_does_not_expand_request_or_command_budgets(){
        let (mut producer,mut reader)=UnixStream::pair().unwrap();
        let body=serde_json::json!({"history":"retained history ".repeat(100_000)});
        let expected=body.clone();
        let bytes=serde_json::to_vec(&Frame::new("snapshot","large-read",body)).unwrap();
        assert!(bytes.len()>REQUEST_FRAME_MAX);
        let thread=std::thread::spawn(move||{producer.write_all(&bytes).unwrap();producer.write_all(b"\n").unwrap();});
        assert_eq!(read_frame(&mut reader).unwrap().body,expected);
        thread.join().unwrap();
        assert!(write_frame(&mut reader,&Frame::new("snapshot","request",expected)).is_err());
        assert!(write_frame(&mut reader,&Frame::new("command","mutation",serde_json::json!({"text":"x".repeat(COMMAND_MAX)}))).is_err());
    }
    #[test]
    fn coalesced_replies_preserve_the_next_frame() {
        let (mut producer,mut reader)=UnixStream::pair().unwrap();
        let mut bytes=serde_json::to_vec(&Frame::new("snapshot","first",serde_json::json!({"text":"界".repeat(4096)}))).unwrap();
        bytes.push(b'\n');
        bytes.extend(serde_json::to_vec(&Frame::new("status","second",serde_json::json!({"ready":true}))).unwrap());
        bytes.push(b'\n');
        let thread=std::thread::spawn(move||producer.write_all(&bytes).unwrap());
        let first=read_frame(&mut reader).unwrap();
        assert_eq!(first.request_id,"first");
        assert_eq!(first.body["text"],"界".repeat(4096));
        assert_eq!(read_frame(&mut reader).unwrap().request_id,"second");
        thread.join().unwrap();
    }
    #[test]
    fn fragmented_frame_and_closed_partial_frame_keep_their_semantics() {
        let (mut producer,mut reader)=UnixStream::pair().unwrap();
        let bytes=serde_json::to_vec(&Frame::new("snapshot","fragmented",serde_json::json!({"text":"界".repeat(20)}))).unwrap();
        let thread=std::thread::spawn(move||{
            for part in bytes.chunks(5){producer.write_all(part).unwrap();}
            producer.write_all(b"\n{\"v\":1").unwrap();
        });
        assert_eq!(read_frame(&mut reader).unwrap().body["text"],"界".repeat(20));
        assert_eq!(read_frame(&mut reader).unwrap_err().kind(),io::ErrorKind::UnexpectedEof);
        thread.join().unwrap();
    }
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
