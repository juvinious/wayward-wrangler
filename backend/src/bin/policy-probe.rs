//! Development-only parity entry point. No service, worker or state writes.
use serde_json::{Value, json};
use std::io::{self, Read};
use waywr_backend::{
    adapters, context_access, context_policy, delta, fake, findings, local_context, native_policy,
    output, packet,
};

fn evaluate(action: &str, value: &Value) -> Result<Value, String> {
    let prior = &value["prior"];
    let packet = &value["packet"];
    let result = &value["result"];
    match action {
        "classify" => findings::classify(
            value["stage"].as_str().ok_or("missing stage")?,
            prior,
            packet,
            result,
            value.get("interpretation").filter(|v| !v.is_null()),
        ),
        "apply" => findings::apply(
            prior,
            &value["classified"],
            value["ref_id"].as_str().ok_or("missing ref_id")?,
            value["stage"].as_str().ok_or("missing stage")?,
            &value["reviewed"],
        ),
        "questions" => {
            context_policy::validate_questions(result, packet)?;
            Ok(json!({"valid":true}))
        }
        "evidence_ids" => Ok(json!(context_policy::evidence_ids(packet)?)),
        "effective_decisions" => Ok(json!(context_policy::effective_decisions(&value["state"])?)),
        "snapshot" => {
            context_policy::validate_snapshot(&value["snapshot"])?;
            Ok(json!({"valid":true}))
        }
        "native_plan" => native_policy::plan(&value["envelope"]),
        "native_digest" => Ok(json!(native_policy::plan_digest(&value["envelope"])?)),
        "native_check" => {
            native_policy::check_explicit_digest(
                &value["envelope"],
                value["supplied_digest"]
                    .as_str()
                    .ok_or("missing supplied digest")?,
            )?;
            Ok(json!({"valid":true}))
        }
        "adapter_schema" => adapters::schema(packet),
        "adapter_prompt" => Ok(json!(adapters::prompt(packet)?)),
        "adapter_verify" => Ok(json!(adapters::verify_role(
            &value["role"],
            value["version_output"]
                .as_str()
                .ok_or("missing version output")?
        )?)),
        "adapter_env" => {
            let source = value["source_env"]
                .as_object()
                .ok_or("missing source env")?
                .iter()
                .map(|(k, v)| Ok((k.clone(), v.as_str().ok_or("invalid env value")?.to_owned())))
                .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
            Ok(json!(adapters::environment(
                value["provider"].as_str().ok_or("missing provider")?,
                &source
            )?))
        }
        "adapter_argv" => Ok(json!(
            adapters::command(
                packet,
                std::path::Path::new(value["directory"].as_str().ok_or("missing directory")?)
            )?
            .argv
        )),
        "adapter_event" => {
            adapters::inspect_event(
                value["provider"].as_str().ok_or("missing provider")?,
                &value["event"],
            )?;
            Ok(json!({"valid":true}))
        }
        "adapter_finish" => {
            let events = value["events"].as_array().ok_or("missing events")?;
            let final_bytes = value["native_final"].as_str().map(str::as_bytes);
            let (result, runtime) = adapters::finish(
                value["provider"].as_str().ok_or("missing provider")?,
                events,
                final_bytes,
            )?;
            Ok(json!({"result":result,"runtime":runtime}))
        }
        "foreign_select" => {
            let project = std::path::Path::new(
                value["source_project"]
                    .as_str()
                    .ok_or("missing source_project")?,
            );
            let state = value["source_state"].as_str().map(std::path::Path::new);
            context_access::SourceBinding::new(project, state)?.select(&value["selector"])
        }
        "active_grants" => {
            let reader = context_access::GrantReader {
                state: value["recipient_state"]
                    .as_str()
                    .ok_or("missing recipient_state")?
                    .into(),
                recipient_key: value["recipient_key"]
                    .as_str()
                    .ok_or("missing recipient_key")?
                    .into(),
            };
            Ok(json!(reader.active_for_task(
                value["task_id"].as_str().ok_or("missing task_id")?
            )?))
        }
        "check_frozen" => {
            let reader = context_access::GrantReader {
                state: value["recipient_state"]
                    .as_str()
                    .ok_or("missing recipient_state")?
                    .into(),
                recipient_key: value["recipient_key"]
                    .as_str()
                    .ok_or("missing recipient_key")?
                    .into(),
            };
            reader.check_frozen(value["imports"].as_array().ok_or("missing imports")?)?;
            Ok(json!({"valid":true}))
        }
        "local_resolve" => {
            let reader = local_context::LocalContext::new(
                std::path::Path::new(value["project"].as_str().ok_or("missing project")?),
                std::path::Path::new(value["state"].as_str().ok_or("missing state")?),
                value["project_key"].as_str().ok_or("missing project key")?,
            )?;
            reader.resolve(
                std::path::Path::new(value["selection"].as_str().ok_or("missing selection")?),
                value["exclude_run"].as_str(),
            )
        }
        "local_check" => {
            let reader = local_context::LocalContext::new(
                std::path::Path::new(value["project"].as_str().ok_or("missing project")?),
                std::path::Path::new(value["state"].as_str().ok_or("missing state")?),
                value["project_key"].as_str().ok_or("missing project key")?,
            )?;
            reader.check(
                &value["snapshot"],
                value["context_digest"].as_str().ok_or("missing digest")?,
                value["exclude_run"].as_str(),
            )?;
            Ok(json!({"valid":true}))
        }
        "review_delta" => Ok(json!(delta::unified(
            value["previous"].as_str().ok_or("missing previous")?,
            value["current"].as_str().ok_or("missing current")?
        ))),
        "packet" => packet::build(
            &value["envelope"],
            &value["state"],
            value["stage"].as_str().ok_or("missing stage")?,
            value["current_text"]
                .as_str()
                .ok_or("missing current text")?,
            value["previous_text"]
                .as_str()
                .ok_or("missing previous text")?,
        ),
        "fake_result" => fake::result(packet),
        "fake_events" => Ok(json!(fake::events(packet)?)),
        "validate_output" => {
            output::validate(result, packet)?;
            Ok(json!({"valid":true}))
        }
        _ => Err("unknown parity action".into()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let action = std::env::args()
        .nth(1)
        .ok_or("usage: policy-probe ACTION < input.json")?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let value: Value = serde_json::from_str(&input)?;
    match evaluate(&action, &value) {
        Ok(output) => println!(
            "{}",
            serde_json::to_string(&json!({"ok":true,"value":output}))?
        ),
        Err(reason) => println!(
            "{}",
            serde_json::to_string(&json!({"ok":false,"error":reason}))?
        ),
    }
    Ok(())
}
