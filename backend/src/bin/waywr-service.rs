//! Explicit project service entry point. Startup/routing is owned by waywr.
use std::{env,path::PathBuf,sync::Arc};
use waywr_backend::session_service::{Config,Service};

fn main(){
    let mut project=None;let mut state=None;let mut key=None;let mut release=None;
    let mut ipc=None;let mut herdr=None;let mut runner=None;let mut frontend=None;
    let mut args=env::args().skip(1);
    let outcome=(||->Result<(),String>{
        while let Some(flag)=args.next(){
            let value=args.next().ok_or(format!("Missing value for {flag}"))?;
            match flag.as_str(){
                "--project"=>project=Some(PathBuf::from(value)),
                "--state-dir"=>state=Some(PathBuf::from(value)),
                "--project-key"=>key=Some(value),
                "--release-id"=>release=Some(value),
                "--ipc"=>ipc=Some(PathBuf::from(value)),
                "--herdr-socket"=>herdr=Some(PathBuf::from(value)),
                "--runner"=>runner=Some(PathBuf::from(value)),
                "--frontend"=>frontend=Some(PathBuf::from(value)),
                _=>return Err(format!("Unknown service argument: {flag}")),
            }
        }
        let config=Config{project:project.ok_or("Missing --project")?,state:state.ok_or("Missing --state-dir")?,
            key:key.ok_or("Missing --project-key")?,release:release.ok_or("Missing --release-id")?,
            ipc:ipc.ok_or("Missing --ipc")?,herdr_socket:herdr.unwrap_or_default(),
            runner:runner.ok_or("Missing --runner")?,frontend:frontend.ok_or("Missing --frontend")?};
        Arc::new(Service::new(config)?).serve()
    })();
    if let Err(error)=outcome{eprintln!("Wayward Wrangler service: {error}");std::process::exit(3);}
}
