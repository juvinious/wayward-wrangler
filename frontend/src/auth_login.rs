//! One local SDK sign-in child. No project IPC, command outbox or transcript.
use serde_json::{Value,json};
use std::{io::{self,BufRead,BufReader,Read,Write},process::{Child,Command,Stdio},sync::mpsc::{self,Receiver,SyncSender},thread};

pub struct Login {
    child:Child,answers:SyncSender<Value>,events:Receiver<Value>,
}
impl Login {
    pub fn start(provider:&str,method:&str)->io::Result<Self>{
        let node=waywr_backend::model_runtime::node().map_err(io::Error::other)?;
        let entry=waywr_backend::model_runtime::entry().map_err(io::Error::other)?;
        let mut command=Command::new(node);command.arg(entry).args(["login-ui",provider,method]);
        Self::spawn(command)
    }
    fn spawn(mut command:Command)->io::Result<Self>{
        let mut child=command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let input=child.stdin.take().ok_or_else(||io::Error::other("Sign-in input unavailable"))?;
        let output=child.stdout.take().ok_or_else(||io::Error::other("Sign-in output unavailable"))?;
        let(tx,events)=mpsc::sync_channel(32);let(answers,rx)=mpsc::sync_channel::<Value>(4);
        thread::spawn(move||{
            let mut input=input;
            while let Ok(value)=rx.recv(){let Ok(mut bytes)=serde_json::to_vec(&value)else{break;};bytes.push(b'\n');if input.write_all(&bytes).and_then(|_|input.flush()).is_err(){break;}}
        });
        thread::spawn(move||{
            let mut reader=BufReader::new(output);let mut completed=false;
            loop{
                let mut bytes=Vec::new();let result=reader.by_ref().take(65537).read_until(b'\n',&mut bytes);
                if !matches!(result,Ok(n) if n>0)||bytes.len()>65536{break;}
                let Ok(value)=serde_json::from_slice::<Value>(&bytes)else{break;};
                if value["type"]=="auth.completed"{completed=true;}
                if tx.send(value).is_err(){return;}
            }
            if !completed{let _=tx.send(json!({"type":"auth.completed","status":"failed"}));}
        });
        Ok(Self{child,answers,events})
    }
    pub fn answer(&self,id:String,value:String)->io::Result<()>{
        if value.len()>4096{return Err(io::Error::other("Sign-in input exceeds 4 KiB"));}
        self.answers.try_send(json!({"type":"answer","id":id,"value":value})).map_err(|_|io::Error::other("Sign-in input unavailable; cancel and try again"))
    }
    pub fn poll(&self)->Option<Value>{self.events.try_recv().ok()}
}
impl Drop for Login {fn drop(&mut self){let _=self.child.kill();let _=self.child.wait();}}

fn browser_command(url:&str)->io::Result<Command>{
    if url.len()>8192||url.chars().any(|c|c.is_control()||c.is_whitespace())||!(url.starts_with("https://")||url.starts_with("http://")){
        return Err(io::Error::other("Provider did not supply an HTTP browser address"));
    }
    // One argument, no shell. URL/verification codes never enter logs or project state.
    let mut command=Command::new("xdg-open");command.arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());Ok(command)
}
pub fn open_browser(url:&str)->io::Result<()>{
    let mut child=browser_command(url)?.spawn()?;
    thread::spawn(move||{let _=child.wait();});Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn browser_handoff_uses_one_http_argument_without_a_shell(){
        let url="https://example.invalid/authorize?state=fixture&redirect_uri=http%3A%2F%2Flocalhost";
        let command=browser_command(url).unwrap();assert_eq!(command.get_program(),"xdg-open");assert_eq!(command.get_args().collect::<Vec<_>>(),vec![std::ffi::OsStr::new(url)]);
        for invalid in ["file:///tmp/fixture","--execute","javascript:fixture","https://example.invalid/\nfixture"]{assert!(browser_command(invalid).is_err());}
    }
}
