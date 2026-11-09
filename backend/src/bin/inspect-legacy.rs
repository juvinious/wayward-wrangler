use std::{env, path::Path};
use waywr_backend::legacy::LegacyReader;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: inspect-legacy STATE_ROOT RUN_ID");
        std::process::exit(2);
    }
    match LegacyReader::open(Path::new(&args[1])).and_then(|reader| reader.inspect(&args[2])) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("serializable report")),
        Err(error) => { eprintln!("{error}"); std::process::exit(1); }
    }
}
