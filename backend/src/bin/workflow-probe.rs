//! Explicit-state Stage 4 comparison driver. It does not route an operator project.
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use waywr_backend::workflow_store::Store;
fn main() {
    let mut args = std::env::args().skip(1);
    let action = args.next().unwrap_or_default();
    let state = PathBuf::from(args.next().unwrap_or_default());
    if !state.join("rust-migration-test-state.json").exists() {
        eprintln!("Explicit isolated-state marker required");
        std::process::exit(3);
    }
    let result = (|| -> Result<Value, String> {
        let store = Store::open(&state)?;
        let run = args.next().ok_or("missing run ID")?;
        match action.as_str() {
            "create" => {
                let path = PathBuf::from(args.next().ok_or("missing envelope")?);
                let envelope: Value =
                    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                store.create(&envelope)
            }
            "status" => store.snapshot(&run),
            "reserve" => store.reserve_stage(&run),
            "apply" => store.apply_saved(&run, &args.next().ok_or("missing stage")?),
            "gate" => store.prepare_gate(&run),
            "answer" => {
                let request = args.next().ok_or("missing request")?;
                let outcome = args.next().ok_or("missing outcome")?;
                let answer = args.next().ok_or("missing answer")?;
                let hash = args.next();
                store.record_answer(&run, &request, &outcome, &answer, hash.as_deref())
            }
            "consume" => store.consume_answer(&run),
            _ => Err("Expected create, status, reserve, apply, gate, answer or consume".into()),
        }
    })();
    match result {
        Ok(value) => println!("{}", json!(value)),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(3)
        }
    }
}
