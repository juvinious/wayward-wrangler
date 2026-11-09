//! Syntax validation is deliberately separate from dispatch. Every public CLI
//! form is parsed before project registration, service startup, or mutation.
use clap::{Arg,ArgAction,Command};

fn opt(name:&'static str)->Arg{Arg::new(name).long(name).num_args(1)}
fn flag(name:&'static str)->Arg{Arg::new(name).long(name).action(ArgAction::SetTrue)}
fn run(name:&'static str)->Command{Command::new(name).arg(Arg::new("run").required(true))}
const STAGES:[&str;5]=["draft","critique-1","revision-1","critique-2","revision-2"];
fn op(name:&'static str)->Command{run(name).arg(opt("operation").required(true).value_parser(STAGES))}

pub fn command()->Command {
    let globals=["project","state-dir","registry-dir","legacy-origin","color","herdr-socket","ipc-dir","release"];
    let mut root=Command::new("waywr").disable_version_flag(true).arg_required_else_help(false)
        .subcommand_required(false).allow_external_subcommands(false);
    for name in globals {
        let argument=if name=="color" {opt(name).value_parser(["auto","always","never"])}else{opt(name)};
        root=root.arg(argument.global(true));
    }
    for name in ["socket","project-key","release-id","outbox-dir","title","ui-token",
        "view-id","view-token","view-task","view-stage","view-attempt"] {
        root=root.arg(opt(name).hide(true));
    }
    for name in ["backend-info","protocol-probe","lost-reply-probe","yes","view-output"] {
        root=root.arg(flag(name).hide(true));
    }
    root=root
        .subcommand(Command::new("init").arg(Arg::new("directory")))
        .subcommand(Command::new("location-rebind"))
        .subcommand(Command::new("preflight").arg(opt("run").required(true)).arg(opt("brief").required(true))
            .arg(opt("profile")).arg(opt("config")).arg(opt("context").required(true))
            .arg(opt("workspace")).arg(opt("socket")))
        .subcommand(Command::new("integration").arg(Arg::new("integration"))
            .arg(Arg::new("action")).arg(opt("destination")).arg(opt("socket")))
        .subcommand(run("open").arg(Arg::new("stage").value_parser(["latest","draft","critique-1","revision-1","critique-2","revision-2"]))
            .arg(opt("artifact").value_parser(["latest","draft","critique-1","revision-1","critique-2","revision-2"]))
            .arg(opt("sha256")))
        .subcommand(Command::new("open-link"))
        .subcommand(run("rebind").arg(opt("workspace").required(true)))
        .subcommand(Command::new("start").arg(opt("run")).arg(opt("brief")).arg(opt("profile"))
            .arg(opt("config")).arg(opt("context")).arg(opt("workspace")).arg(opt("socket"))
            .arg(opt("authorize-native")))
        .subcommand(Command::new("demo").arg(opt("run").required(true)).arg(opt("brief"))
            .arg(opt("profile")).arg(opt("config")).arg(opt("context"))
            .arg(opt("workspace")).arg(opt("socket")).arg(flag("exit-at-choice"))
            .arg(flag("record-choice-and-exit")).arg(opt("owner-generation")).arg(opt("owner-token")))
        .subcommand(Command::new("open-demo").arg(opt("run").required(true)).arg(opt("brief").required(true))
            .arg(opt("profile")).arg(opt("config")).arg(opt("context")).arg(opt("workspace"))
            .arg(opt("socket")).arg(opt("authorize-native")))
        .subcommand(run("supervise").arg(opt("socket").required(true)).arg(opt("interval"))
            .arg(opt("max-attempts")).arg(flag("once")))
        .subcommand(run("supervisor-install").arg(opt("socket").required(true)))
        .subcommand(run("notices"))
        .subcommand(Command::new("resume").arg(Arg::new("run")))
        .subcommand(Command::new("status").arg(Arg::new("run")).arg(flag("watch")).arg(flag("json")))
        .subcommand(run("workers"))
        .subcommand(run("show").arg(opt("artifact").value_parser(["latest","draft","critique-1","revision-1","critique-2","revision-2"])))
        .subcommand(op("logs"))
        .subcommand(op("observe").arg(opt("viewer").value_parser(["terminal","herdr"]))
            .arg(opt("socket")))
        .subcommand(op("cancel"))
        .subcommand(run("decide").arg(opt("request").required(true)).arg(opt("answer-file").required(true))
            .arg(flag("record-only")))
        .subcommand(run("approve").arg(opt("request").required(true)).arg(opt("sha256").required(true))
            .arg(flag("record-only")))
        .subcommand(run("reject").arg(opt("request").required(true)).arg(opt("reason-file").required(true))
            .arg(flag("record-only")))
        .subcommand(Command::new("session").subcommand_required(true)
            .subcommand(Command::new("status")).subcommand(Command::new("stop"))
            .subcommand(Command::new("migrate").arg(opt("release")))
            .subcommand(Command::new("bind").arg(opt("release").required(true)))
            .subcommand(Command::new("upgrade").arg(opt("release").required(true)))
            .subcommand(Command::new("rollback").arg(opt("release").required(true)))
            .subcommand(Command::new("reconcile-request-routing").about("Save request role choices from one retained human message; no approval or turn")
                .arg(opt("request").required(true)).arg(opt("message").required(true)))
            .subcommand(Command::new("repair-answer-history").about("Append a verified saved-answer correction in a stopped project; never replay input")
                .arg(opt("event").required(true)).arg(opt("request").required(true)).arg(opt("misrouted-events")))
            .subcommand(Command::new("recover-coordinator").arg(Arg::new("task").required(true)))
            .subcommand(Command::new("retry-start").arg(Arg::new("task").required(true)))
            .subcommand(Command::new("command-status").arg(Arg::new("command_id").required(true)))
            .subcommand(Command::new("command-replay").arg(Arg::new("command_id").required(true))))
        .subcommand(Command::new("legacy-inspect").arg(Arg::new("run").required(true)))
        .subcommand(Command::new("adopt-saved-draft").arg(Arg::new("run").required(true)))
        .subcommand(Command::new("adopt-provisional-revision").arg(Arg::new("run").required(true)))
        .subcommand(Command::new("adopt-reviewed-critique").arg(Arg::new("run").required(true)).arg(opt("interpretation").required(true)))
        .subcommand(Command::new("bundle-build").arg(opt("release-id").required(true))
            .arg(opt("output").required(true)).arg(opt("source-root").required(true))
            .arg(opt("frontend").required(true)).arg(opt("service").required(true))
            .arg(opt("runner").required(true)))
        .subcommand(Command::new("bundle-install").arg(opt("bundle").required(true)).arg(opt("prefix").required(true)).arg(flag("select-default")));
    for name in ["probe-start","probe-resume","probe-cancel","probe-observe"] {
        root=root.subcommand(Command::new(name).arg(Arg::new("name").required(true)).arg(opt("workspace"))
            .arg(opt("socket")).arg(opt("seconds")).arg(opt("mode").value_parser(["success","failure"])).arg(flag("detach")));
    }
    root
}

pub fn validate(args:&[String])->Result<bool,String>{
    let mut argv=vec!["waywr".to_owned()];argv.extend_from_slice(args);
    match command().try_get_matches_from(argv){
        Ok(_)=>Ok(true),
        Err(error) if matches!(error.kind(),clap::error::ErrorKind::DisplayHelp|clap::error::ErrorKind::DisplayVersion)=>{
            print!("{error}");Ok(false)
        }
        Err(error)=>Err(error.to_string()),
    }
}

#[cfg(test)]mod tests{
    use super::*;
    fn accepts(args:&[&str])->bool{validate(&args.iter().map(|s|s.to_string()).collect::<Vec<_>>()).is_ok()}
    #[test]fn malformed_flags_never_reach_dispatch(){
        assert!(!accepts(&["--project","/tmp/p","status","run","--artifcat","draft"]));
        assert!(!accepts(&["--project","/tmp/p","approve","run","--request","r"]));
        assert!(!accepts(&["--project","/tmp/p","status","--watch=perhaps"]));
        assert!(accepts(&["--project","/tmp/p","show","run","--artifact","draft"]));
        assert!(accepts(&["--project","/tmp/p","decide","run","--request","d","--answer-file","answer.txt","--record-only"]));
        assert!(accepts(&["--project","/tmp/p","observe","run","--operation","draft",
            "--viewer","herdr","--socket","/tmp/private-herdr.sock"]));
        assert!(!accepts(&["--project","/tmp/p","observe","run","--operation","draft",
            "--viewer","interactive"]));
    }
    #[test]fn ordinary_r9_forms_are_explicitly_parsed(){
        let supported: &[&[&str]]=&[
            &["--project","/tmp/p","init"],
            &["--project","/tmp/p","location-rebind"],
            &["preflight","--run","r","--brief","brief.md","--context","none","--profile","profile.toml"],
            &["integration","herdr","install","--destination","/tmp/plugin.py","--socket","/tmp/herdr.sock"],
            &["open","r","--artifact","revision-2","--sha256","abc"],
            &["rebind","r","--workspace","w2"],
            &["start","--run","r","--brief","brief.md","--context","none"],
            &["supervise","r","--socket","/tmp/herdr.sock","--once"],
            &["supervisor-install","r","--socket","/tmp/herdr.sock"],
            &["notices","r"],&["resume","r"],&["status","r","--watch","--json"],
            &["workers","r"],&["show","r","--artifact","draft"],
            &["logs","r","--operation","draft"],&["observe","r","--operation","draft"],
            &["cancel","r","--operation","draft"],
            &["decide","r","--request","q","--answer-file","answer.txt","--record-only"],
            &["approve","r","--request","q","--sha256","abc","--record-only"],
            &["reject","r","--request","q","--reason-file","reason.txt","--record-only"],
            &["session","status"],&["session","stop"],
            &["session","bind","--release","release-id"],
            &["session","upgrade","--release","release-id"],
            &["session","rollback","--release","release-id"],
            &["session","recover-coordinator","task-id"],
            &["session","retry-start","task-id"],
            &["session","command-status","cli-id"],&["session","command-replay","cli-id"],
        ];
        for args in supported {assert!(accepts(args),"Rejected valid syntax: {args:?}");}
        let refused: &[&[&str]]=&[
            &["start","--rnu","r"],&["show","r","--artifact","unknown"],
            &["logs","r","--operation","unknown"],&["approve","r","--request","q"],
            &["decide","r","--answer-file","answer.txt"],&["cancel","r"],
            &["--color","neon","status"],&["status","r","--artifact","draft"],
            &["session","upgrade"],&["integration","herdr","install","--destinaton","/tmp/plugin.py"],
        ];
        for args in refused {assert!(!accepts(args),"Accepted malformed syntax: {args:?}");}
    }
}
