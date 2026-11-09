use std::{io::Write,path::PathBuf};
fn main() {
    let mut args = std::env::args().skip(1);
    let action = args.next().unwrap_or_default();
    let result = match action.as_str() {
        "development-check-child" => waywr_backend::development::check_child(&PathBuf::from(args.next().unwrap_or_default())),
        "fake-child" => {
            let packet = PathBuf::from(args.next().unwrap_or_default());
            let counter = PathBuf::from(args.next().unwrap_or_default());
            waywr_backend::runner::fake_child(&packet, &counter)
        }
        "run" => {
            let mut directory = None;
            let mut token = None;
            let mut state = None;
            while let Some(key) = args.next() {
                match key.as_str() {
                    "--operation-dir" => directory = args.next().map(PathBuf::from),
                    "--launch-token" => token = args.next(),
                    "--state-dir" => state = args.next().map(PathBuf::from),
                    _ => {
                        eprintln!("Unknown runner argument: {key}");
                        std::process::exit(2);
                    }
                }
            }
            match (directory, state, token) {
                (Some(directory), Some(state), Some(token)) => {
                    let panic_dir=directory.clone();
                    std::panic::set_hook(Box::new(move|info|{
                        eprintln!("Wayward Wrangler runner panic: {info}");
                        if let Ok(mut file)=std::fs::OpenOptions::new().write(true).create_new(true)
                            .open(panic_dir.join("runner-panic.txt")){
                            let _=writeln!(file,"{info}");let _=file.sync_all();
                        }
                    }));
                    waywr_backend::runner::run(&directory, &state, &token)
                }
                _ => Err("Runner requires --operation-dir, --state-dir and --launch-token".into()),
            }
        }
        "host-run" => {
            let mut directory=None;let mut state=None;let mut attempt=None;
            while let Some(key)=args.next(){
                match key.as_str(){
                    "--operation-dir"=>directory=args.next().map(PathBuf::from),
                    "--state-dir"=>state=args.next().map(PathBuf::from),
                    "--attempt-id"=>attempt=args.next(),
                    _=>{eprintln!("Unknown host runner argument: {key}");std::process::exit(2);}
                }
            }
            match (directory,state,attempt){
                (Some(directory),Some(state),Some(attempt))=>
                    waywr_backend::runner::run_host(&directory,&state,&attempt),
                _=>Err("Host runner requires --operation-dir, --state-dir and --attempt-id".into()),
            }
        }
        _ => Err("Expected run, host-run or fake-child".into()),
    };
    if let Err(error) = result {
        eprintln!("Wayward Wrangler runner: {error}");
        std::process::exit(3);
    }
}
