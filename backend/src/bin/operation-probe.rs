//! Isolated migration test driver; it never selects a default project or server.
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use waywr_backend::{herdr::Herdr, operation::{self, Ensure,OwnerFence}};

fn main(){
    let mut args=std::env::args().skip(1);
    let action=args.next().unwrap_or_default();
    let state=PathBuf::from(args.next().unwrap_or_default());
    let directory=PathBuf::from(args.next().unwrap_or_default());
    let result=(||->Result<Value,String>{
        if !state.join("rust-migration-test-state.json").exists(){return Err("Explicit isolated-state marker required".into());}
        if action=="prepare"{
            let packet:Value=serde_json::from_slice(&fs::read(args.next().ok_or("missing packet path")?).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            let target:Value=serde_json::from_slice(&fs::read(args.next().ok_or("missing target path")?).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            return operation::prepare(&state,&directory,&packet,&target,None).map_err(|e|e.to_string());
        }
        if action=="status"{return Ok(operation::inspect(&directory,None));}
        if action=="cancel"{return operation::cancel(&directory,"test-controller").map_err(|e|e.to_string());}
        if action!="ensure"{return Err("Expected prepare, ensure, status or cancel".into());}
        let op=waywr_backend::execution::read(&directory.join("operation.json"))?;
        let socket=op["target"]["socket"].as_str().ok_or("missing frozen socket")?;
        let control=Herdr::new(socket);
        let runner=std::env::current_exe().map_err(|e|e.to_string())?.with_file_name("waywr-runner");
        let fence=OwnerFence{state:state.clone(),generation:None};
        Ensure{state:&state,directory:&directory,runner_binary:&runner,control:&control,fence:&fence,
            launch_gate:None,before_launch:None}
            .run().map_err(|e|format!("{:?}: {}",e.kind,e.detail))
    })();
    match result {Ok(value)=>println!("{}",json!(value)),Err(error)=>{eprintln!("{error}");std::process::exit(3)}}
}
