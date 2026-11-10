use super::{
    editor::{Composer, Draft, EditResult},
    input_history::InputHistory,
    rich::{markdown_lines, Tone},
    scenario::{First, Journal, Second, Target, SOURCE_HASH},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
use serde_json::Value;
use tachyonfx::{fx, Effect, EffectTimer, Interpolation};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    None,
    Help,
    Status,
    Tasks,
    Scope,
    ScopeEditor,
    Question,
    QuestionConfirm,
    Queue,
    Consultation,
    Steering,
    Review,
    Artifact,
    Inspect,
    Worker,
    Settings,
    Target,
    Detach,
    Pause,
    Cancel,
    UnknownCommand,
    History,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Composer,
    Send,
    Multiline,
    Target,
    Activity,
    Conversation,
    Drawer,
    Header,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeAction {
    Next, Acknowledge, Authorize, Previous, Edit, RequestRevision, Close,
}

#[derive(Clone, Copy)]
struct Command {
    name: &'static str,
    description: &'static str,
    shortcut: &'static str,
}
const COMMANDS: &[Command] = &[
    Command { name:"resume",description:"Resume this paused session",shortcut:"" },
    Command {name:"model",description:"Choose a model from the full catalog",shortcut:""},
    Command {name:"thinking",description:"Choose the model's thinking level",shortcut:""},
    Command {name:"status",description:"Inspect session, model, queue and usage",shortcut:""},
    Command { name:"reply",description:"Answer the current development coordinator question",shortcut:"" },
    Command {
        name: "help",
        description: "Help for this view",
        shortcut: "F1",
    },
    Command {
        name: "tasks",
        description: "Choose task and inspect workers",
        shortcut: "F6",
    },
    Command {
        name: "settings",
        description: "Coordinator settings and local appearance",
        shortcut: "F3",
    },
    Command {
        name: "new",
        description: "Target a new task",
        shortcut: "",
    },
    Command {
        name: "inspect",
        description: "Inspect selected task and context",
        shortcut: "",
    },
    Command {
        name: "output",
        description: "Read live or retained coordinator and worker output",
        shortcut: "",
    },
    Command {
        name: "herdr",
        description: "Open selected output in an optional read-only Herdr tab",
        shortcut: "",
    },
    Command {
        name: "plan",
        description: "Review proposed scope",
        shortcut: "",
    },
    Command {
        name: "confirm",
        description: "Open scope authorization panel",
        shortcut: "",
    },
    Command {
        name: "answer",
        description: "Open pending question",
        shortcut: "",
    },
    Command {
        name: "consult",
        description: "Review and authorize one pending second-agent consultation",
        shortcut: "",
    },
    Command {
        name: "open",
        description: "Read bundle or /open FILE for one candidate file",
        shortcut: "",
    },
    Command {
        name: "approve",
        description: "Open final review",
        shortcut: "",
    },
    Command {
        name: "reject",
        description: "Reject candidate with reason",
        shortcut: "",
    },
    Command {
        name: "steer",
        description: "Steer unstarted queued work",
        shortcut: "",
    },
    Command {
        name: "cancel",
        description: "Cancel selected task",
        shortcut: "",
    },
    Command {name:"interrupt",description:"Interrupt the foreground coordinator; keep working or leave",shortcut:"Esc / Ctrl-C when empty"},
    Command {
        name: "grants",
        description: "Inspect selected context imports",
        shortcut: "",
    },
    Command {
        name: "include",
        description: "Unavailable: import creation is outside this demo",
        shortcut: "",
    },
    Command {
        name: "revoke",
        description: "Unavailable: no demo import to revoke",
        shortcut: "",
    },
    Command {
        name: "detach",
        description: "Close interface; authorized work continues",
        shortcut: "Ctrl-Q",
    },
    Command {name:"queue",description:"Inspect queued/held messages; release or withdraw explicitly",shortcut:"F7"},
    Command {name:"discuss",description:"Explicitly discuss the inspected task's request",shortcut:""},
    Command {
        name: "exit",
        description: "Leave interface; service-owned work continues",
        shortcut: "",
    },
    Command {name:"pause",description:"Stop new dispatch; current calls settle separately",shortcut:""},
    Command {name:"quit",description:"Leave interface; service-owned work continues",shortcut:"Ctrl-Q"},
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveIntent {
    Dialogue(Value), SaveScope(Value), AuthorizeScope(Value), ContinueAnswer, SubmitTask(String), Plan, Inspect, Authorize, Consult, ApplySteering, ReplaceCoordinator, Answer(String), Approve,
    Reject(String), OpenArtifact, OpenArtifactNext, OpenFile(String), Output, HerdrView, Cancel, Pause, Detach,
}

pub struct LiveState {
    pub project: String,
    pub tasks: Vec<Value>,
    pub conversation: Option<Value>,
    pub plan: Option<Value>,
    pub detail: Option<Value>,
    pub artifact: Option<String>,
    pub preview_file: Option<String>,
    pub reviewed_sha: Option<String>,
    pub reviewed_identity: Option<(String,String,String,String)>,
    pub artifact_next_offset:u64,
    pub artifact_complete:bool,
    pub worker_task_id: Option<String>,
    pub operations: Vec<Value>,
    worker_activities: Vec<Value>,
    activity_task_id: Option<String>,
    activity_fetched: Option<Instant>,
    pub worker_index: usize,
    pub worker_output: String,
    pub worker_cursor: Value,
    pub worker_terminal: bool,
    pub worker_at_end: bool,
    pub worker_limit: Option<Value>,
    pub phase: String,
    pub last_seq: u64,
    pub intents: VecDeque<LiveIntent>,
}

impl LiveState {
    fn selected(&self, index: usize) -> Option<&Value> { self.tasks.get(index) }
    fn status(&self, index: usize) -> &str {
        self.selected(index).and_then(|v| v["status"].as_str()).unwrap_or("No task")
    }
    fn title(&self, index: usize) -> &str {
        self.selected(index).and_then(|v| v["title"].as_str()).unwrap_or("No task")
    }
}

pub struct App {
    pub journal: Journal,
    pub composer: Composer,
    pub data_dir: PathBuf,
    pub modal: Modal,
    pub focus: Focus,
    pub menu: bool,
    pub menu_index: usize,
    pub modal_action: usize,
    pub question_choice: Option<usize>,
    pub question_text: String,
    pub reject_reason: String,
    pub notice: String,
    card_seqs: std::collections::BTreeMap<u64,String>,
    card_presenting: Option<String>,
    card_action: usize,
    delivery_turn: Option<String>,
    finished_turns: VecDeque<String>,
    finished_inputs: VecDeque<String>,
    draft_reply:Option<Value>,
    draft_replies:std::collections::BTreeMap<String,Value>,
    history_reply:Option<Value>,
    question_modal_key:Option<Value>,
    pub detached: bool,
    interrupt_pending:Option<(String,Option<String>)>,
    pub next_due: Option<Instant>,
    pub effect: Option<Effect>,
    pub effect_started: Instant,
    pub dirty: bool,
    pub persist_pending: bool,
    pub saved_at: Instant,
    pub screen_width: u16,
    pub modal_scroll: u16,
    modal_scroll_limit: u16,
    pub history_index: usize,
    pub input_history: InputHistory,
    pub viewport_top: usize,
    pub viewport_total: usize,
    pub viewport_height: usize,
    pub viewport_width: u16,
    pub cursor_visible: bool,
    cursor_changed: Instant,
    activity_changed: Instant,
    pub live: Option<LiveState>,
    pending_primary_task: Option<String>,
    pub submission_draft: Option<Draft>,
    pub scope_edit:Option<Value>,
    pub scope_editor:Composer,
    pub scope_field:usize,
    scope_ack:Option<String>,
    scope_step:usize,
    pub conversation_subject:String,
    pub queue_index:usize,
    restored_queue_input:Option<Value>,
    last_queue_restore:Option<String>,
    pub conversation_revision_request:bool,
    question_confirmation:Option<Value>,
    project_edit:Option<Value>,provider_edit:Option<Value>,provider_section:usize,provider_index:usize,provider_row:usize,provider_field:usize,provider_typing:bool,provider_grant:bool,provider_interrupt:bool,
    provider_picker:Option<CatalogPicker>,provider_follow:bool,
    provider_activation:Option<ActivationReview>,
}

impl App {
    pub fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        let journal = Journal::load(&data_dir)?;
        Ok(Self::with_journal(journal, data_dir, None))
    }
    pub fn new_live(project: String, draft: Draft) -> Self {
        let mut journal = Journal::default();
        journal.scenario = "live".into();
        journal.target = Target::NewTask;
        journal.drafts.insert(Target::NewTask.key().into(), draft);
        let live = LiveState { project, tasks: Vec::new(), conversation:None, plan: None, detail: None,
            artifact: None, preview_file:None, reviewed_sha: None,reviewed_identity:None,artifact_next_offset:0,artifact_complete:false,
            worker_task_id:None,operations:Vec::new(),worker_activities:Vec::new(),activity_task_id:None,activity_fetched:None,
            worker_index:0,worker_output:String::new(),worker_cursor:Value::Null,
            worker_terminal:false,worker_at_end:false,worker_limit:None,phase: "running".into(), last_seq: 0,
            intents: VecDeque::new() };
        let mut app=Self::with_journal(journal, PathBuf::new(), Some(live));
        app.composer.area.set_placeholder_text("Describe a new task request…");
        app
    }
    fn with_journal(journal: Journal, data_dir: PathBuf, live: Option<LiveState>) -> Self {
        let draft = journal
            .drafts
            .get(journal.target.key())
            .cloned()
            .unwrap_or_default();
        let mut app = Self {
            journal,
            composer: Composer::from_draft(draft),
            data_dir,
            modal: Modal::None,
            focus: Focus::Composer,
            menu: false,
            menu_index: 0,
            modal_action: 0,
            question_choice: None,
            question_text: String::new(),
            reject_reason: String::new(),
            notice: String::new(),
            card_seqs:Default::default(),card_presenting:None,card_action:0,
            delivery_turn:None,finished_turns:VecDeque::new(),finished_inputs:VecDeque::new(),
            draft_reply:None,draft_replies:Default::default(),
            history_reply:None,
            question_modal_key:None,
            detached: false,
            interrupt_pending:None,
            next_due: None,
            effect: None,
            effect_started: Instant::now(),
            dirty: true,
            persist_pending: false,
            saved_at: Instant::now(),
            screen_width: 80,
            modal_scroll: 0,
            modal_scroll_limit: 0,
            history_index: 0,
            input_history: InputHistory::default(),
            viewport_top: 0,
            viewport_total: 0,
            viewport_height: 0,
            viewport_width: 78,
            cursor_visible: true,
            cursor_changed: Instant::now(),
            activity_changed: Instant::now(),
            live,
            pending_primary_task: None,
            submission_draft: None,
            scope_edit:None,scope_editor:Composer::default(),scope_field:1,scope_ack:None,scope_step:0,conversation_subject:"discussion".into(),queue_index:0,restored_queue_input:None,last_queue_restore:None,conversation_revision_request:false,question_confirmation:None,project_edit:None,provider_edit:None,provider_section:0,provider_index:0,provider_row:0,provider_field:0,provider_typing:false,provider_grant:false,provider_interrupt:false,provider_picker:None,provider_follow:true,provider_activation:None,
        };
        if app.live.is_none() { app.schedule(); }
        if app.journal.effects == "subtle" {
            app.effect = Some(fx::fade_to_fg(
                Tone::for_theme(&app.journal.theme).accent,
                EffectTimer::from_ms(180, Interpolation::Linear),
            ));
        }
        app
    }
    pub fn take_intent(&mut self) -> Option<LiveIntent> {
        self.live.as_mut()?.intents.pop_front()
    }
    fn intent(&mut self, intent: LiveIntent) {
        if let Some(live)=self.live.as_mut() { live.intents.push_back(intent); }
        self.mark();
    }
    pub fn apply_snapshot(&mut self, body: &Value) {
        let previous_scope=self.current_scope().map(Self::scope_identity);
        let previous_subject_task = self.conversation_scope_task_id().map(str::to_owned);
        let old_queue_id=self.queue_rows().get(self.queue_index).map(Self::queue_identity);
        let first_conversation=self.live.as_ref().is_some_and(|live|live.conversation.is_none())
            && body.get("conversation").is_some();
        if first_conversation && self.input_history.is_empty() {
            if let Some(messages) = body["conversation"]["messages"].as_array() {
                for message in messages.iter().filter(|message| message["role"] == "user") {
                    if let (Some(id), Some(text)) = (message["message_id"].as_str(), message["text"].as_str()) {
                        // Existing message records establish sent text, not whether
                        // slash text was typed as a command. Retain it as literal.
                        self.input_history.seed(format!("message-{id}"), Draft {
                            text: text.into(), literal_paste: true,
                            row: text.split('\n').count() - 1,
                            col: text.rsplit('\n').next().unwrap_or("").chars().count(),
                            ..Draft::default()
                        });
                    }
                }
            }
        }
        let old_question=self.pending_conversation_question().and_then(|q|q["question_id"].as_str()).map(str::to_owned);
        let new_question=body["conversation"]["questions"].as_array().and_then(|items|items.iter().find(|q|q["status"]=="open"&&(body["conversation"]["continuous"].is_null()||q["scope"]["subject_id"]==self.conversation_subject)))
            .and_then(|q|q["question_id"].as_str()).map(str::to_owned);
        if old_question!=new_question&&!(self.coordination_mode()&&self.modal==Modal::Question){self.question_choice=None;self.question_text.clear();}
        if first_conversation && self.journal.target==Target::NewTask{
            self.journal.target=Target::Coordinator;
            self.journal.drafts.insert(Target::Coordinator.key().into(),self.composer.snapshot());
        }
        if let Some(live)=self.live.as_mut() {
            let selected_id=live.selected(self.journal.selected_task)
                .and_then(|v|v["task_id"].as_str()).map(str::to_owned);
            live.tasks=body["tasks"].as_array().cloned().unwrap_or_default();
            live.conversation=body.get("conversation").cloned();
            if live.conversation.is_some(){self.composer.area.set_placeholder_text(if live.conversation.as_ref().is_some_and(|v|!v["chat"].is_null()){"Message the coordinator…"}else{"Message the mock coordinator…"});}
            live.phase=body["session"]["phase"].as_str().unwrap_or("unknown").to_owned();
            self.journal.selected_task=selected_id.and_then(|id|live.tasks.iter().position(|v|v["task_id"]==id))
                .unwrap_or_else(||self.journal.selected_task.min(live.tasks.len().saturating_sub(1)));
        }
        // Follow a newly saved scope in this topic, without replacing an open
        // review or an explicit task-list selection on every refresh.
        if self.modal == Modal::None && self.focus != Focus::Drawer
            && self.conversation_scope_task_id() != previous_subject_task.as_deref()
        {
            self.select_conversation_scope();
        }
        self.provider_refresh_saved();
        if self.interrupt_pending.as_ref().is_some_and(|(turn,_)|
            !body["conversation"]["turns"].as_array().into_iter().flatten().any(|r|r["turn_id"]==*turn&&r["status"]=="reserved") &&
            !body["conversation"]["continuous"]["queue"].as_array().into_iter().flatten().any(|r|r["turn_id"]==*turn&&r["status"]=="reserved")) {
            self.interrupt_pending=None;
        }
        self.restore_proposal_cards();
        self.restore_candidate_cards();
        if self.coordination_mode()&&previous_scope!=self.current_scope().map(Self::scope_identity){self.scope_ack=None;self.scope_step=0;self.card_action=0;}
        if self.modal==Modal::Queue{let rows=self.queue_rows();if let Some(i)=old_queue_id.and_then(|id|rows.iter().position(|r|Self::queue_identity(r)==id)){self.queue_index=i;}else{self.queue_index=self.queue_index.min(rows.len().saturating_sub(1));self.modal_action=0;}}
        if body["conversation"]["history_truncated"]==true{self.notice="Older conversation is retained in project records; this view shows the bounded recent window".into();}
        if let Some(events)=body["recent_events"].as_array() {
            for event in events { self.apply_event(event); }
        }
        if let Some(messages)=body["conversation"]["messages"].as_array(){
            for item in messages{
                let Some(seq)=item["seq"].as_u64() else{continue};
                if self.journal.messages.iter().any(|message|message.seq==seq){continue;}
                let speaker=match item["role"].as_str(){Some("user")=>"You",
                    Some("consultant")=>"Consultant",_=>"Coordinator"};
                self.journal.messages.push(super::scenario::Message{at:item["at"].as_f64(),seq,speaker:speaker.into(),
                    text:item["text"].as_str().unwrap_or("").into(),
                    task_id:item["task_id"].as_str().unwrap_or("").into(),task_version:0,
                    simulated:true,source_sha256:String::new(),activity:None});
            }
            self.journal.messages.sort_by_key(|message|message.seq);
            if self.journal.messages.len()>200{
                self.journal.messages.drain(..self.journal.messages.len()-200);
            }
            if let Some(last)=self.journal.messages.last(){
                if let Some(live)=self.live.as_mut(){live.last_seq=live.last_seq.max(last.seq);}
            }
        }
        self.mark();
    }
    fn event_activity(kind:&str,body:&Value)->Option<super::scenario::ActivityStatus>{
        use super::scenario::ActivityStatus::*;
        Some(match kind{
            "development.review_outcome"=>if body["review_stale"]==true{Attention}else if body["status"]=="reviewed"{match body["review"]["requires_changes"].as_bool(){Some(false)=>Success,Some(true)=>Attention,None=>Activity}}else if body["status"]=="failed"{Failure}else{Activity},
            "development.combined_checks"=>match body["checks"]["result"]["passed"].as_bool(){Some(true)=>Success,Some(false)=>Failure,None=>Attention},
            "development.unit_outcome"=>match body["status"].as_str(){Some("failed"|"output-invalid")=>Failure,Some("blocked")=>Attention,Some("uncertain")=>Uncertain,Some("cancelled")=>Cancelled,_=>Activity},
            "development.operation_failed"|"development.worker_report_retained"=>Failure,
            "development.attention"|"development.decision_rejected"=>Attention,
            "development.recovery"|"development.recovery_resolved"|"development.reassessment"=>if body["recovery"]["phase"]=="resolved"{Activity}else if body["recovery"]["cause"]=="uncertain_execution"{Uncertain}else if body["recovery"]["phase"]=="blocked"||body["recovery"]["decision_needed"]==true{Attention}else{Activity},
            "development.delivered"=>Success,
            kind if kind.starts_with("development.")||kind=="scope.proposed"=>Activity,
            _=>return None,
        })
    }
    fn review_outcome_text(body:&Value)->String{
        let outcome=if body["review_stale"]==true{"Review stale"}else if body["status"]=="reviewed"{match body["review"]["requires_changes"].as_bool(){Some(false)=>"Review complete",Some(true)=>"Review needs changes",None=>"Review recorded"}}else if body["status"]=="failed"{"Review failed"}else{"Review recorded"};
        format!("{outcome} · {}",body["review"]["summary"].as_str().unwrap_or("Inspect findings"))
    }
    pub fn apply_event(&mut self, event: &Value) {
        let Some(live)=self.live.as_mut() else { return; };
        let Some(seq)=event["seq"].as_u64() else { return; };
        if seq<=live.last_seq {
            // Rehydrate an older display cache from the same immutable event.
            // Do not append a message, reinterpret authority or alter the event.
            let kind=event["type"].as_str().unwrap_or("");
            let activity=Self::event_activity(kind,&event["body"]);
            if let Some(m)=self.journal.messages.iter_mut().find(|m|m.seq==seq&&m.speaker=="Waywr"&&m.activity.is_none()){
                if activity.is_some(){m.activity=activity;if kind=="development.review_outcome"{m.text=Self::review_outcome_text(&event["body"]);}self.mark();}
            }
            return;
        }
        live.last_seq=seq;
        let kind=event["type"].as_str().unwrap_or("event");
        if kind.starts_with("conversation.input_")&&event["body"]["subject_id"]==self.conversation_subject{
            self.notice=match kind{"conversation.input_applied"=>"Steering applied to the agent context","conversation.input_held"=>"Steering was not applied; retained in F7 for editing or withdrawal","conversation.input_delivering"=>"Steering is entering the agent context","conversation.input_withdrawn"=>"Queued steering withdrawn",_=>"Steering pending"}.into();
        }
        self.coordination_event(kind,&event["body"]);
        if self.coordination_mode()&&kind=="conversation.proposal_card"{if let Some(id)=event["body"]["card_id"].as_str(){self.card_seqs.insert(seq,id.into());while self.card_seqs.len()>64{self.card_seqs.pop_first();}}}
        if self.coordination_mode()&&kind=="conversation.card_presented"{self.mark();return;}
        if self.development_mode()&&matches!(kind,"development.progress"|"development.coordinator_decision"|"development.unit_base"|"development.git_intent"|"development.git_applied"|"development.integration_recorded"|"development.integration_complete"|"development.knowledge_updated"|"development.requirement_delta"|"development.obligations_disclosed"){self.mark();return}
        if self.continuous_mode()&&matches!(kind,"conversation.turn_reserved"|"conversation.delta"|"conversation.recovery_acknowledged"){self.mark();return}
        let task=event["task_id"].as_str().unwrap_or("task");
        let body=&event["body"];
        if self.chat().is_some() && (matches!(kind,"conversation.delta"|"conversation.turn_reserved")|| !self.continuous_mode()&&kind=="conversation.turn_completed"){if kind=="conversation.delta" && (self.journal.scroll_back>0 || self.journal.scroll_anchor_seq.is_some()){self.journal.unread=self.journal.unread.saturating_add(1);}self.mark();return;}
        if self.continuous_mode()&&kind=="conversation.turn_stopped"&&!body["pause"].is_null(){
            if let Some(c)=self.live.as_mut().and_then(|l|l.conversation.as_mut()){
                c["continuous"]["pause"]=body["pause"].clone();c["chat"]["status"]=serde_json::json!(if body["recoverable"]==true{"recoverable"}else{"blocked"});
                if let Some(rows)=c["continuous"]["queue"].as_array_mut(){rows.retain(|r|r["turn_id"]!=body["turn_id"]);for r in rows{if r["status"]=="queued"{r["status"]=serde_json::json!("held");}}}
            }
        }
        let (speaker,text)=match kind {
            "conversation.proposal_card"=>{if self.journal.messages.iter().any(|m|m.seq==seq){self.mark();return;} ("Waywr",Self::proposal_card_text(body))},
            "conversation.provider_grant" if self.coordination_mode()=>("Waywr",Self::provider_grant_text(body)),
            "conversation.provider_choice" if self.coordination_mode()=>("Waywr",format!("Provider choice saved: {} → {}. Future attempts; coordinator handover uses {}.",body["role"].as_str().unwrap_or("role"),body["profile_id"].as_str().unwrap_or("profile"),body["switch_mode"].as_str().unwrap_or("after current turn"))),
            "conversation.proposal_disposition"=>("Waywr",format!("Proposal v{} {}. No work authorized.",body["base"]["revision"],if body["disposition"]=="reject"{"rejected"}else{"deferred"})),
            "conversation.input_result" if self.coordination_mode()=>{let out=&body["outcome"];("Waywr",if out["status"]=="rejected"{format!("Input needs attention: {}. Original input retained.",out["message"].as_str().unwrap_or("Inspect this request"))}else if !out["clarification"].is_null(){if out["clarification"]["kind"]=="provider"{out["clarification"]["reason"].as_str().unwrap_or("Name one exact profile or use F3").to_owned()}else{format!("{} · Select Approve below for this exact saved proposal, or describe a change.",out["clarification"]["reason"].as_str().unwrap_or("Approval intent is unclear"))}}else if body["route"]=="answer_conversation"{if out["question"]["disposition"]=="defer"{"Question deferred; it remains unresolved."}else if out["continuation_saved"]==true{"Answer saved. Preparing the proposal when the coordinator is available."}else{"Answer saved."}.into()}else if body["route"]=="authorize_scope"{"Proposal and operating policy approved. Starting authorized work.".into()}else{self.mark();return;})},
            "conversation.message"=>{
                let speaker=match body["role"].as_str(){Some("user")=>"You",Some("consultant")=>"Consultant",_=>"Coordinator"};
                (speaker,body["text"].as_str().unwrap_or("").to_owned())
            },
            "conversation.authorized"=>("Waywr",format!("Coordinator allowance saved · Up to {} conversation calls. Worker execution and consultation remain unavailable.",body["call_limit"].as_u64().unwrap_or(3))),
            "conversation.turn_completed" if self.continuous_mode()=>{if self.coordination_mode()&&body["action_disposition"]!="stale"{self.mark();return;}if body["scope_saved"]==true{self.mark();return} ("Waywr",if body["action_disposition"]=="stale"{"Not applied · saved scope or decisions changed. Last valid scope preserved."}else{"Conversation complete · no scope change saved."}.into())},
            "conversation.question_suggestion"=>("Waywr","Suggested formal answer ready · Record it when ready, or keep discussing. No decision was saved.".into()),
            "conversation.answer" if self.coordination_mode()=>("Waywr",format!("Answer saved: {}",body["answer"].as_str().unwrap_or("Saved decision"))),
            "conversation.answer" if self.continuous_mode()=>("Waywr","Formal question decision recorded · No model call or scope authorization.".into()),
            "conversation.answer_continuation" if self.coordination_mode()=>{self.mark();return;},
            "conversation.withdrawn"=>("Waywr","Queued message withdrawn · No call spent.".into()),
            "conversation.turn_stopped" if self.continuous_mode()=>("Waywr",if body["status"]=="interrupted"{format!("Interrupted · {}",if body["recoverable"]==true{"Message saved; send a new message to continue. Older queued messages remain held."}else{"Inspect retained evidence before continuing."})}else{format!("Reply unavailable · {}. {}",body["reason"].as_str().unwrap_or("The coordinator could not provide a valid reply"),if body["recoverable"]==true{"Message saved; send a new message to continue. Older queued messages remain held."}else{"Inspect retained evidence before continuing."})}),
            "conversation.cancel_requested"=>("Waywr","Coordinator interruption requested · Partial output and held allowance are retained.".into()),
            "conversation.input_pending"=>("Waywr","Steering pending · The active agent will receive it at its next boundary.".into()),
            "conversation.input_delivering"=>("Waywr","Applying steering to the active agent context.".into()),
            "conversation.input_applied"=>("Waywr","Steering applied to the active agent context.".into()),
            "conversation.input_held"=>("Waywr","Steering was not applied · Retained in F7 for editing or withdrawal.".into()),
            "conversation.input_withdrawn"=>("Waywr","Pending steering withdrawn.".into()),
            "conversation.mock_delta"=>("Waywr","Mock coordinator working from selected project context…".into()),
            "conversation.question" if self.coordination_mode()=>("Waywr",format!("Your answer is needed: {} Reply here.",body["question"].as_str().unwrap_or("question"))),
            "conversation.question"=>("Waywr",format!("Your answer is needed: {} Open the visible Answer question action; saving an answer does not spend a call.",body["question"].as_str().unwrap_or("question"))),
            "conversation.action_proposed"=>("Waywr",match body["kind"].as_str(){
                Some("request_consultation")=>"Second-agent consultation ready. Review its scope and authorize one mock consultation call.".into(),
                Some("propose_steering")=>"Steering suggestion ready for an unadmitted task. Review it before creating a linked replacement proposal.".into(),
                _=>"Coordinator action ready for review.".into()}),
            "conversation.steering_applied"=>("Waywr","Steering applied to a linked replacement proposal. The original task remains in history; review the new scope before authorizing any work.".into()),
            "conversation.turn_blocked"|"conversation.turn_stopped"=>("Waywr",format!("Coordinator {} · {}. Open /inspect for retained evidence; no call will retry automatically.",body["status"].as_str().unwrap_or("blocked"),body["reason"].as_str().unwrap_or("call stopped"))),
            "scope.proposed" if self.coordination_mode()=>("Waywr",format!("Proposal revision {} saved. Its application card follows; reply approve or describe a change.",body["revision"])),
            "scope.proposed"=>("Waywr",format!("Proposal revision {} saved. Open Review scope; {}.",body["revision"],if self.development_mode(){"approval is required before workers start"}else{"execution remains unavailable"})),
            "providers.configuration_blocked"=>("Waywr",format!("Provider configuration needs attention · {}",body["attention"].as_str().unwrap_or("Correct F3 settings; no model process launched"))),
            "providers.preferences_saved"|"providers.preferences_imported"=>("Waywr","Provider preferences saved · Future attempts only; current coordinator uses its saved binding.".into()),
            "providers.switch_requested"|"providers.capacity_switch_requested"=>("Waywr",format!("Coordinator switch requested · {} · {}",body["target"]["profile_id"].as_str().unwrap_or("selected profile"),body["mode"].as_str().unwrap_or("after current turn"))),
            "providers.switch_waiting"=>("Waywr","Coordinator switch waiting · Current calls settle; workers continue.".into()),
            "providers.switch_applied"=>("Waywr",format!("Coordinator switched · {} · Conversation and development binding preserved.",body["target"]["profile_id"].as_str().unwrap_or("selected profile"))),
            "providers.switch_blocked"=>("Waywr",format!("Coordinator switch blocked · {}",body["reason"].as_str().unwrap_or("Inspect uncertain execution"))),
            "providers.switch_cancelled"=>("Waywr","Coordinator switch cancelled · Existing binding retained.".into()),
            "providers.policy_amended"=>("Waywr","Provider policy amended · Scope approval and existing work preserved.".into()),
            "providers.task_override"=>("Waywr",format!("Task profile override saved · {} · Unreserved attempts only.",body["title"].as_str().unwrap_or("task"))),
            "development.candidate_ready"=>("Waywr",Self::candidate_text(body)),
            "development.candidate_feedback"=>("Waywr",format!("Candidate {} · {}\n{}",body["action"],body["commit"],body["text"])),
            "development.recovery"|"development.recovery_resolved"|"development.reassessment"=>("Waywr",waywr_backend::development_recovery::presentation(&body["recovery"])),
            "development.review_started"=>("Waywr",format!("Review queued · {} · Exact frozen commits; no editing.",body["title"].as_str().unwrap_or("review"))),
            "development.review_outcome"=>("Waywr",Self::review_outcome_text(body)),
            "development.coordinator_reserved"=>("Waywr","Coordinator reviewing progress; independent workers continue.".into()),
            "development.started"|"development.authorized"=>("Waywr","Development authorized · Tasks can run under the approved scope and operating policy.".into()),
            "development.assignment_retired"=>("Waywr",format!("Obsolete assignment retired · {}\n{}\nOriginal evidence and obligations retained.",body["title"].as_str().or(body["name"].as_str()).unwrap_or("task"),body["retirement"]["reason"].as_str().unwrap_or("Coordinator adapted the plan"))),
            "development.result_branch_updated"=>("Waywr",format!("Combined task branch · {}\nPreview checkout: {}\nMain advances only after delivery gates pass.",body["result_branch"].as_str().unwrap_or("pending"),body["integration_worktree"].as_str().unwrap_or("Inspect"))),
            "development.assignment_planned"=>("Waywr",format!("Task planned · {}",body["title"].as_str().or_else(||body["name"].as_str()).unwrap_or("task"))),
            "development.assignment_started"=>("Waywr",format!("Worker assignment queued · {}",body["title"].as_str().or_else(||body["name"].as_str()).unwrap_or("task"))),
            "development.unit_integrated"=>("Waywr",format!("Task integrated · {}",body["title"].as_str().or_else(||body["name"].as_str()).unwrap_or("Reviewed submission"))),
            "development.unit_accepted"=>("Waywr",format!("Task accepted · {}",body["title"].as_str().or_else(||body["name"].as_str()).unwrap_or("task"))),
            "development.integration_intent"=>("Waywr","Integration queued · The coordinator reviewed the exact submitted commit.".into()),
            "development.combined_checks"=>("Waywr",format!("Combined checks {} · Inspect the command results and exact integration commit.",if body["checks"]["result"]["passed"]==true{"passed"}else{"failed"})),
            "development.delivered"=>("Waywr",if body["delivery"]["kind"]=="github-pr"{format!("Delivery complete · PR merged: {} · local target {}. Inspect the exact merged/candidate commits and history.",body["delivery"]["url"],body["delivery"]["local_target"])}else{format!("Delivery complete · Combined checks passed and target {} advanced. Inspect the exact commit and request history.",body["main_branch"].as_str().unwrap_or("main"))}),
            "development.promotion_intent"=>("Waywr",format!("Final checks accepted · {} completion queued.",body["kind"].as_str().unwrap_or("selected"))),
            "development.publication_wait"=>("Waywr",format!("PR awaiting checks/review · {}\nNormal GitHub gates apply; the local target has not advanced.",body["publication"]["pr"]["url"])),
            "development.resource_retired"=>("Waywr",format!("Owned resource retired · {}\nExact Git objects retained at {}. Receipts/history remain available.",body["path"],body["archive_ref"])),
            "development.resource_retained"=>("Waywr",format!("Resource retained · {}\n{}",body["path"],body["reason"])),
            "development.recovery_context"=>("Waywr","Operational recovery recorded · The coordinator will reassess retained work.".into()),
            "development.resumed"=>("Waywr","Development resumed · Existing work and approval preserved.".into()),
            "development.workspace_reconciled"=>("Waywr","Integration workspace recovered · Python cache bytes retained with a manifest; application source unchanged.".into()),
            "development.capacity_wait"=>("Waywr","Waiting for provider capacity · Work retained under the saved wait/fallback policy.".into()),
            "development.capacity_resumed"=>("Waywr","Provider retry scheduled · Retained work will be continued.".into()),
            "development.decision_rejected"=>("Waywr","Coordinator action needs revision · Replanning automatically.".into()),
            "development.operation_failed"=>("Waywr","Execution call failed · Evidence saved; coordinator will replan.".into()),
            "development.dependency_notification"=>("Waywr","Dependency update recorded · Active workers continue to a safe checkpoint.".into()),
            "development.dependency_combined"=>("Waywr","Dependency satisfied · Combined integration includes the required producer.".into()),
            "development.unit_outcome"=>("Waywr",format!("Worker {} · {}{}",body["name"].as_str().unwrap_or("task"),body["status"].as_str().unwrap_or("settled"),body["worker_report"]["summary"].as_str().map(|s|format!(" · {s}")).unwrap_or_default())),
            "development.worker_report_retained"=>("Waywr",format!("Worker {} · failed · {}",body["name"].as_str().unwrap_or("task"),body["worker_report"]["report"]["summary"].as_str().unwrap_or("Failure report retained; inspect evidence"))),
            "development.attention"=>("Waywr",format!("Development needs attention · {} · Use /reply for a saved coordinator question; /steer records an in-scope change.",body["attention"].as_str().unwrap_or("Inspect retained evidence"))),
            "development.answer"=>("You",format!("Answer to development question · {}",body["text"].as_str().unwrap_or("Answer saved"))),
            "scope.authorized"=>("Waywr",body["label"].as_str().unwrap_or("Scope authorized · execution unavailable").into()),
            "scope.authorization_superseded"=>("Waywr","Scope revision changed. Review the latest revision before authorizing; saved decisions, if any, remain in history.".into()),
            "scope.suggestion_stale"=>("Waywr","Based on an earlier scope; suggestion not applied. Your saved edits and decisions remain intact.".into()),
            "task.proposed"=>("Waywr","Task proposal ready. Open Review scope; no worker call is authorized yet.".into()),
            "task.queued" => ("You",body["text"].as_str().unwrap_or("Task request saved").to_owned()),
            "proposal.ready" => ("Waywr",format!("Task proposal ready · {task}. Open the scope to review it; no worker call is authorized yet.")),
            "task.authorized" => ("Waywr",format!("Task authorized · {task}. Queued for supervised execution.")),
            "task.state_changed" if body.get("coordinator_execution").is_some() => {
                let detail=body["detail"].as_str().unwrap_or("Coordinator stopped without a proposal");
                let summary=if detail.contains("Bootstrap tab contains foreground work") {
                    "Herdr reported the project worker tab busy before launch; no proposal was created."
                }else if body["coordinator_execution"]=="pre-send-refusal" {
                    "Herdr refused the worker tab before launch; no proposal was created."
                }else {
                    "The coordinator call did not produce a usable proposal."
                };
                ("Waywr",format!("Coordinator stopped · {summary} This task will not retry automatically. Open /inspect for the exact reason and retained evidence."))
            },
            "task.state_changed" if matches!(body["status"].as_str(),Some("blocked"|"uncertain"|"output-invalid")) =>
                ("Waywr",format!("Task needs attention · {}. Open /inspect for retained evidence and the supported next action.",body["status"].as_str().unwrap_or("blocked"))),
            "task.state_changed" => ("Waywr",format!("{} · {}",body["status"].as_str().unwrap_or("Updated"),body["detail"].as_str().unwrap_or(task))),
            "task.recovery_started" => ("Waywr",format!("Coordinator recovery queued · {task}. The prior failure remains inspectable.")),
            "task.active" => ("Waywr",format!("Active · {task}. Open /output for live worker streams.")),
            "session.state_changed" => ("Waywr",format!("Session · {}",body["phase"].as_str().unwrap_or("state changed"))),
            "session.attention" if body.is_null() => ("Waywr","Service recovery · Project supervision resumed.".into()),
            "session.attention" => ("Waywr",format!("Project supervision {} · {} Open /inspect for retained diagnostics and the next action.",
                body["status"].as_str().unwrap_or("needs attention"),
                if body["status"]=="stopped"{"Scheduling stopped after three failed reconciliation attempts."}
                else{"Reconciliation needs attention."})),
            _ => ("Waywr",format!("{task} · {kind}")),
        };
        if self.journal.scroll_back>0 || self.journal.scroll_anchor_seq.is_some() { self.journal.unread=self.journal.unread.saturating_add(1); }
        self.journal.messages.push(super::scenario::Message { at:event["at"].as_f64(),seq, speaker:speaker.into(), text,
            task_id:task.into(), task_version:0, simulated:true, source_sha256:String::new(),activity:Self::event_activity(kind,body) });
        while self.journal.messages.len()>200 { self.journal.messages.remove(0); }
        self.mark();
    }
    pub fn apply_plan(&mut self, body: Value) {
        if let Some(live)=self.live.as_mut() { live.plan=Some(body); }
        self.modal=Modal::Scope;
        self.modal_action=0;
        self.notice="Scope loaded from this project's service. Opening it does not authorize work.".into();
        self.mark();
    }
    pub fn apply_detail(&mut self, body: Value) {
        let row = self
            .live
            .as_ref()
            .and_then(|live| live.selected(self.journal.selected_task));
        let selected = row.and_then(|task| task["task_id"].as_str());
        if selected != body["task"]["task_id"].as_str()
            || row.is_none_or(|task| task["version"] != body["task"]["version"] ||
                task["state_revision"] != body["task"]["state_revision"])
        {
            return;
        }
        if self.scope_mode() && self.current_scope().map(Self::scope_identity)!=body.get("scope").map(Self::scope_identity){
            self.scope_ack=None;
            self.scope_step=0;
            if self.modal==Modal::Scope {
                self.modal_action=0;
                self.modal_scroll=0;
                self.modal_scroll_limit=0;
            }
        }
        let requested = self.pending_primary_task.take().as_deref() == selected;
        if let Some(live) = self.live.as_mut() {
            let identity = Self::candidate_identity(&live.project, &body);
            let previous = live
                .detail
                .as_ref()
                .and_then(|detail| Self::candidate_identity(&live.project, detail));
            if previous != identity {
                live.reviewed_identity = None;
                live.reviewed_sha = None;
                live.artifact = None;
                live.artifact_next_offset = 0;
                live.artifact_complete = false;
            }
            live.detail = Some(body);
        }
        if self.scope_mode()&&self.modal==Modal::Scope&&self.modal_action>=self.scope_actions().len(){self.modal_action=0;}
        if self.modal == Modal::Review && !self.candidate_read() && self.modal_action > 1 {
            self.modal_action = 1;
        }
        if requested {
            self.open_selected_action();
        }
        self.mark();
    }
    pub fn pending_action_failed(&mut self) {
        self.pending_primary_task = None;
    }
    fn candidate_identity(
        project: &str,
        detail: &Value,
    ) -> Option<(String, String, String, String)> {
        if detail["pending"]["kind"] != "final-review" {
            return None;
        }
        Some((
            project.to_owned(),
            detail["task"]["task_id"].as_str()?.to_owned(),
            detail["pending"]["request_id"].as_str()?.to_owned(),
            detail["pending"]["artifact_sha256"].as_str()?.to_owned(),
        ))
    }
    pub fn apply_artifact(&mut self, body: String, sha: String) {
        if let Some(live)=self.live.as_mut() {
            live.artifact=Some(body);
            live.reviewed_sha=Some(sha);
        }
        self.modal=Modal::Artifact;
        self.mark();
    }
    pub fn apply_artifact_page(&mut self,body:&Value,text:&str){
        let Some(live)=self.live.as_mut() else{return;};
        live.preview_file=None;
        let expected=live.detail.as_ref().and_then(|detail|detail["pending"]["artifact_sha256"].as_str());
        if expected!=body["sha256"].as_str() ||
            live.detail.as_ref().and_then(|detail|detail["task"]["task_id"].as_str())!=body["task_id"].as_str() ||
            body["offset"].as_u64()!=Some(live.artifact_next_offset){
            self.notice="Candidate changed or page arrived out of order; approval remains blocked".into();
            self.mark();return;
        }
        let artifact=live.artifact.get_or_insert_with(String::new);
        artifact.push_str(text);
        const CACHE:usize=2*1024*1024;
        if artifact.len()>CACHE{
            let mut remove=artifact.len()-CACHE;
            while !artifact.is_char_boundary(remove){remove+=1;}
            artifact.drain(..remove);
            artifact.insert_str(0,"[Earlier candidate pages were shown and evicted; reopen to read again]\n");
        }
        live.artifact_next_offset=body["next_offset"].as_u64().unwrap_or(live.artifact_next_offset);
        live.artifact_complete=body["complete"]==true;
        if live.artifact_complete{
            live.reviewed_identity=live.detail.as_ref().and_then(|detail|Self::candidate_identity(&live.project,detail));
            live.reviewed_sha=body["sha256"].as_str().map(str::to_owned);
        }
        self.modal=Modal::Artifact;self.mark();
    }
    pub fn apply_file_page(&mut self,body:&Value,text:&str){
        let Some(live)=self.live.as_mut() else{return;};
        let expected=live.detail.as_ref().and_then(|detail|detail["pending"]["artifact_sha256"].as_str());
        let selected=live.detail.as_ref().and_then(|detail|detail["task"]["task_id"].as_str());
        if expected!=body["manifest_sha256"].as_str() || selected!=body["task_id"].as_str() ||
            body["complete"]!=true || body["offset"]!=0{
            self.notice="File preview changed or incomplete; bundle approval remains blocked".into();self.mark();return;
        }
        live.preview_file=body["artifact"].as_str().map(str::to_owned);
        live.artifact=Some(text.to_owned());
        live.artifact_complete=true;
        live.reviewed_sha=None;
        self.modal=Modal::Artifact;
        self.notice="Verified one candidate file. Read the full bundle before approval".into();self.mark();
    }
    pub fn apply_operation_list(&mut self,body:&Value){
        let selected=self.live.as_ref().and_then(|live|live.selected(self.journal.selected_task))
            .and_then(|task|task["task_id"].as_str()).map(str::to_owned);
        if selected.as_deref()!=body["task_id"].as_str(){return;}
        if let Some(live)=self.live.as_mut(){
            let operations=body["operations"].as_array().cloned().unwrap_or_default();
            live.worker_activities=operations.clone();
            live.activity_task_id=selected.clone();
            live.activity_fetched=Some(Instant::now());
            if body["active_only"]==true{self.dirty=true;return;}
            if live.worker_task_id.as_deref()!=selected.as_deref(){
                live.worker_task_id=selected;
                live.worker_index=operations.len().saturating_sub(1);
                live.worker_output.clear();live.worker_cursor=Value::Null;
                live.worker_terminal=false;live.worker_at_end=false;live.worker_limit=None;
            }else if live.operations.is_empty() && !operations.is_empty(){
                live.worker_index=operations.len()-1;
            }else{live.worker_index=live.worker_index.min(operations.len().saturating_sub(1));}
            live.operations=operations;
        }
        self.mark();
    }
    pub fn apply_worker_output(&mut self,body:&Value,text:&str){
        let Some(live)=self.live.as_mut() else{return;};
        let selected=live.operations.get(live.worker_index);
        if live.worker_task_id.as_deref()!=body["task_id"].as_str() ||
            selected.and_then(|item|item["stage"].as_str())!=body["stage"].as_str(){return;}
        let page=&body["page"];
        if live.worker_cursor==page["cursor"] && !text.is_empty(){return;}
        let changed=live.worker_cursor!=page["cursor"] || !text.is_empty() ||
            live.worker_terminal!=(page["terminal"]==true) ||
            live.worker_limit.as_ref()!=page.get("output_limit").filter(|value|!value.is_null());
        if !changed{return;}
        live.worker_output.push_str(text);
        const CACHE:usize=2*1024*1024;
        if live.worker_output.len()>CACHE{
            let mut remove=live.worker_output.len()-CACHE;
            while !live.worker_output.is_char_boundary(remove){remove+=1;}
            live.worker_output.drain(..remove);
            live.worker_output.insert_str(0,"[Earlier output available through /output paging or CLI logs]\n");
        }
        live.worker_cursor=page["cursor"].clone();
        live.worker_terminal=page["terminal"]==true;
        live.worker_at_end=if page["format"]=="indexed-v1"{
            live.worker_cursor["sequence"].as_u64().unwrap_or(0)>page["high_water"].as_u64().unwrap_or(0)
        }else{live.worker_cursor["offset"]==page["high_water"]};
        live.worker_limit=page.get("output_limit").filter(|value|!value.is_null()).cloned();
        self.mark();
    }
    pub fn worker_read_request(&self)->Option<Value>{
        let live=self.live.as_ref()?;
        if live.worker_terminal && live.worker_at_end{return None;}
        let operation=live.operations.get(live.worker_index)?;
        if operation["attempt_id"].as_str().is_none(){return None;}
        Some(serde_json::json!({"task_id":live.worker_task_id,"stage":operation["stage"],
            "attempt_id":operation["attempt_id"],"cursor":live.worker_cursor,"limit":16*1024}))
    }
    fn mark(&mut self) {
        self.dirty = true;
        self.persist_pending = true;
    }
    fn scroll_by(&mut self, up: bool, lines: usize) {
        let max_top = self.viewport_total.saturating_sub(self.viewport_height);
        let desired = if up {
            self.viewport_top.saturating_sub(lines)
        } else {
            self.viewport_top.saturating_add(lines).min(max_top)
        };
        self.viewport_top = desired;
        if desired >= max_top {
            self.journal.scroll_anchor_seq = None;
            self.journal.scroll_anchor_line = 0;
            self.journal.scroll_back = 0;
            self.journal.unread = 0;
        } else {
            self.journal.scroll_anchor_seq = None;
            self.journal.scroll_anchor_line = 0;
            let mut start = 0usize;
            for message in self
                .journal
                .messages
                .iter()
                .filter(|m| Self::visible_message(m))
            {
                let count = Paragraph::new(self.message_text(message, self.viewport_width))
                    .wrap(Wrap { trim: false })
                    .line_count(self.viewport_width.max(1));
                if desired < start + count {
                    self.journal.scroll_anchor_seq = Some(message.seq);
                    self.journal.scroll_anchor_line =
                        (desired - start).min(u16::MAX as usize) as u16;
                    break;
                }
                start += count;
            }
            self.journal.scroll_back = (max_top - desired).min(u16::MAX as usize) as u16;
        }
        self.mark();
    }
    pub fn save(&mut self) -> std::io::Result<()> {
        if self.live.is_some() { self.persist_pending=false; return Ok(()); }
        self.journal
            .drafts
            .insert(self.journal.target.key().into(), self.composer.snapshot());
        self.journal.save(&self.data_dir)?;
        self.saved_at = Instant::now();
        self.persist_pending = false;
        Ok(())
    }
    fn schedule(&mut self) {
        let pending = matches!(
            self.journal.first,
            First::PendingDelivery
                | First::Coordinator
                | First::Authorized
                | First::Drafting
                | First::Critiquing
                | First::AnswerUnknown
                | First::Revising
        ) || (matches!(self.journal.first, First::Approved | First::Rejected)
            && matches!(
                self.journal.second,
                Second::AuthorizedQueued | Second::Active | Second::Recovering | Second::Recovered
            ));
        self.next_due = if pending {
            Some(Instant::now() + Duration::from_secs(3))
        } else {
            None
        };
    }
    pub fn tick(&mut self) -> bool {
        let animate = self.journal.effects == "subtle" && self.journal.theme != "mono";
        if self.active_work().is_some() && self.activity_changed.elapsed() >= Duration::from_millis(if animate { 250 } else { 1000 }) {
            self.activity_changed = Instant::now();
            self.dirty = true;
        }
        if self.focus == Focus::Composer
            && self.modal == Modal::None
            && self.cursor_changed.elapsed() >= Duration::from_millis(550)
        {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_changed = Instant::now();
            self.dirty = true; // Presentation only: no journal write for a blink.
        }
        if self.next_due.is_some_and(|when| Instant::now() >= when) {
            return self.advance();
        }
        false
    }
    pub fn advance(&mut self) -> bool {
        if self.live.is_some() { return false; }
        if self.journal.advance() {
            self.schedule();
            self.mark();
            if matches!(self.journal.first, First::Question | First::Review) {
                self.start_effect(Color::Yellow, 250);
            } else if matches!(self.journal.first, First::Approved) {
                self.start_effect(Color::Green, 150);
            }
            true
        } else {
            false
        }
    }
    fn start_effect(&mut self, color: Color, ms: u32) {
        if self.journal.effects != "subtle" {
            return;
        }
        self.effect = Some(fx::fade_to_fg(
            color,
            EffectTimer::from_ms(ms, Interpolation::Linear),
        ));
        self.effect_started = Instant::now();
        self.mark();
    }
    pub fn effects_running(&self) -> bool {
        self.effect.as_ref().is_some_and(|e| !e.done())
    }
    fn restore_recalled_input(&mut self, draft: Draft) {
        self.draft_reply=if self.input_history.is_recalling(){self.coordination_mode().then(||self.coordination_context("recall"))}else{self.history_reply.take().filter(|v|!v.is_null())};
        self.composer = Composer::from_draft(draft);
        if self.live.is_some() {
            self.composer.area.set_placeholder_text(if self.chat().is_some() {
                "Message the coordinator…"
            } else { "Describe a new task request…" });
        }
        // Recalling slash text only fills the composer. Enter remains explicit;
        // pasted slash messages retain their literal flag.
        self.menu = false;
        self.mark();
    }
    fn open(&mut self, modal: Modal) {
        if modal==Modal::Question{self.question_modal_key=self.pending_conversation_question().map(|q|serde_json::json!({"question_id":q["question_id"],"revision":q["scope"]["question_revision"]}));}
        if modal == Modal::History {
            if !self.input_history.is_recalling(){self.history_reply=Some(self.draft_reply.clone().unwrap_or(Value::Null));}
            self.history_index = 0;
            if self.live.is_none() && self.input_history.is_empty() {
                for message in self.journal.messages.iter().filter(|message| message.speaker.starts_with("You ·")) {
                    self.input_history.seed(format!("journal-{}", message.seq), Draft {
                        text: message.text.clone(), literal_paste: true, ..Draft::default()
                    });
                }
            }
        }
        if modal==Modal::Settings&&self.provider_state()["schema_version"]==1{self.provider_edit=Some(self.provider_draft());self.provider_typing=false;self.provider_grant=false;self.provider_interrupt=false;self.provider_picker=None;self.provider_follow=true;self.provider_activation=None;}
        self.modal = modal;
        self.modal_action = 0;
        self.modal_scroll = 0;
        self.modal_scroll_limit = 0;
        self.menu = false;
        self.mark();
        if matches!(modal, Modal::Tasks | Modal::Settings) {
            self.start_effect(Color::Cyan, 100);
        }
    }
    fn set_target(&mut self, target: Target) {
        if !self.development_mode() && self.chat().is_some() && target!=Target::Coordinator && !(self.scope_mode()&&target==Target::NewTask){self.notice="Only coordinator conversation is available; draft preserved".into();self.mark();return;}
        if let Some(original) = self.input_history.take_original() {
            self.composer = Composer::from_draft(original);
            self.draft_reply=self.history_reply.take().filter(|v|!v.is_null());
        }
        let old = if self.continuous_mode(){format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target))}else{self.journal.target.key().to_owned()};
        if let Some(context)=self.draft_reply.take(){self.draft_replies.insert(old.clone(),context);}
        self.journal.drafts.insert(old, self.composer.snapshot());
        self.journal.target = target;
        if self.continuous_mode(){
          self.conversation_revision_request=false;
          if target==Target::NewTask{self.conversation_subject=format!("discussion-{}",waywr_backend::execution::random_token().unwrap_or_else(|_|"local".into()));}
        }
        self.composer = Composer::from_draft(
            self.journal
                .drafts
                .get(&if self.continuous_mode(){format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(target))}else{target.key().to_owned()})
                .cloned()
                .unwrap_or_default(),
        );
        if self.continuous_mode(){self.draft_reply=self.draft_replies.get(&format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(target))).cloned();}
        self.freeze_unbound_draft();
        if let Some(live)=&self.live {
            self.composer.area.set_placeholder_text(if live.conversation.as_ref().is_some_and(|v|!v["chat"].is_null()){"Message the coordinator…"}else if live.conversation.is_some(){
                match target{Target::Coordinator=>"Message the coordinator…",Target::NewTask=>"Discuss a new request…",Target::SteerFirst|Target::SteerSecond=>"Steer the selected task…",Target::ReplyFirst=>"Answer the coordinator’s development question…"}
            }else{"Describe a new task request…"});
        }
        self.focus = Focus::Composer;
        self.mark();
    }
    fn selected_status(&self) -> String {
        if let Some(live)=&self.live {
            if self.development_mode(){if let Some(w)=self.development_view()["workflows"].as_array().and_then(|a|a.iter().find(|w|w["work_id"]==live.selected(self.journal.selected_task).map_or(Value::Null,|t|t["task_id"].clone()))){return format!("Development · {}",w["status"].as_str().unwrap_or("running"));}}
            if self.scope_mode()&&live.status(self.journal.selected_task)=="scope-authorized"{return "Scope authorized · execution unavailable".into();}
            if live.tasks.is_empty() && live.conversation.is_some(){
                if self.pending_conversation_question().is_some(){return "awaiting-answer".into();}
                if self.pending_consultation().is_some(){return "consultation-ready".into();}
                if let Some(chat)=self.chat(){return chat["status"].as_str().unwrap_or("blocked").into();}
                return "discussion".into();
            }
            return live.status(self.journal.selected_task).to_owned();
        }
        if self.journal.selected_task == 1 {
            match self.journal.second {
                Second::None => "No second task".into(),
                Second::Proposed => "Scope ready".into(),
                Second::AuthorizedQueued => "Authorized · queued".into(),
                Second::Active => "Executing".into(),
                Second::Recovering => "Recovering".into(),
                Second::Recovered => "Recovered".into(),
                Second::Uncertain => "Execution uncertain".into(),
                Second::Cancelled => "Cancelled".into(),
            }
        } else {
            match self.journal.first {
                First::Empty => "No task".into(),
                First::PendingDelivery => "Sending… not yet saved".into(),
                First::Coordinator => "Request saved · coordinator working".into(),
                First::Suggested => "Task suggestion ready".into(),
                First::Proposed => "Scope ready".into(),
                First::Authorized => "Authorized · queued".into(),
                First::Drafting => "Drafting".into(),
                First::Critiquing => "Independent review".into(),
                First::Question => "Your answer needed".into(),
                First::AnswerUnknown => "Answer saved · delivery reply unknown".into(),
                First::Revising => "Revising".into(),
                First::Review => "Final review ready".into(),
                First::Approved => "Approved (demo)".into(),
                First::Rejected => "Rejected (demo)".into(),
                First::Cancelled => "Cancelled (demo)".into(),
            }
        }
    }
    fn selected_title(&self) -> String {
        if let Some(live)=&self.live {
            if live.tasks.is_empty() && live.conversation.is_some(){return "Coordinator conversation".into();}
            return live.title(self.journal.selected_task).to_owned();
        }
        if self.journal.selected_task == 1 {
            "Manual walkthrough".into()
        } else {
            "Clarify Decision Log choices".into()
        }
    }
    fn pending_choice(&self) -> Option<&Value> {
        let live = self.live.as_ref()?;
        let selected = live.selected(self.journal.selected_task)?;
        let detail = live.detail.as_ref()?;
        (selected["status"] == "awaiting-input"
            && detail["task"]["task_id"] == selected["task_id"]
            && detail["task"]["version"] == selected["version"]
            && detail["task"]["state_revision"] == selected["state_revision"]
            && detail["pending"]["kind"] == "choice")
            .then_some(&detail["pending"])
    }
    pub fn chat(&self)->Option<&Value>{self.live.as_ref()?.conversation.as_ref()?.get("chat").filter(|v|!v.is_null())}
    pub fn pending_conversation_question(&self)->Option<&Value>{
        self.live.as_ref()?.conversation.as_ref()?["questions"].as_array()?.iter().find(|q|q["status"]=="open"&&(!self.continuous_mode()||q["scope"]["subject_id"]==self.conversation_subject))
    }
    pub fn continuous_mode(&self)->bool{self.live.as_ref().and_then(|l|l.conversation.as_ref()).is_some_and(|v|!v["continuous"].is_null())}
    pub fn dialogue_state(&self)->Value{self.live.as_ref().and_then(|l|l.conversation.as_ref()).map(|v|v["continuous"].clone()).unwrap_or(Value::Null)}
    fn conversation_scope_task_id(&self) -> Option<&str> {
        self.live.as_ref()?.conversation.as_ref()?["continuous"]["subjects"]
            .as_array()?.iter().find(|subject| subject["subject_id"] == self.conversation_subject)?
            ["task_id"].as_str()
    }
    fn select_task(&mut self, index: usize) {
        if self.journal.selected_task != index {
            self.journal.selected_task = index;
            self.pending_primary_task = None;
            self.scope_ack = None;
            self.scope_step = 0;
            self.modal_action = 0;
            self.modal_scroll = 0;
            self.modal_scroll_limit = 0;
        }
        self.mark();
    }
    fn select_conversation_scope(&mut self) -> bool {
        let Some(index) = self.conversation_scope_task_id().and_then(|id|
            self.live.as_ref()?.tasks.iter().position(|task| task["task_id"] == id))
        else { return false; };
        self.select_task(index);
        true
    }
    pub fn question_suggestion(&self)->Option<&Value>{
      if !self.continuous_mode(){return None}self.live.as_ref()?.conversation.as_ref()?["actions"].as_array()?.iter().find(|a|a["kind"]=="suggest_question_disposition"&&a["body"]["subject_id"]==self.conversation_subject)
    }
    fn queue_rows(&self)->Vec<Value>{self.dialogue_state()["queue"].as_array().cloned().unwrap_or_default()}
    fn queue_identity(row:&Value)->Value{row.get("input_id").unwrap_or(&row["turn_id"]).clone()}
    fn queue_editable(row:&Value)->bool{matches!(row["status"].as_str(),Some("queued"|"pending"|"held"))}
    fn queue_withdraw(&mut self,row:&Value,restore:bool){
        if restore&&!self.composer.text().is_empty(){self.notice="Keep or clear the current draft before restoring a queued input".into();self.mark();return;}
        if !Self::queue_editable(row){self.notice="This input has already entered the agent context".into();self.mark();return;}
        let payload=if row["input_id"].is_string(){serde_json::json!({"action":"conversation.withdraw_input","payload":{"input_id":row["input_id"],"subject_id":row["subject_id"],"restore_to_editor":restore}})}else{
            serde_json::json!({"action":"withdraw_queued","payload":{"turn_id":row["turn_id"],"subject_id":row["subject_id"],"restore_to_editor":restore}})};
        self.intent(LiveIntent::Dialogue(payload));self.notice="Queue change pending; applied inputs cannot be edited".into();self.mark();
    }
    pub fn restore_queue_input(&mut self,value:Value){
        if value["text"].as_str().is_none_or(|t|t.len()>32768){return;}
        if let Some(id)=value["source_command"].as_str(){
            if self.last_queue_restore.as_deref()==Some(id){return;}self.last_queue_restore=Some(id.into());
        }
        self.restored_queue_input=Some(value);self.restore_pending_queue_input();
    }
    fn restore_pending_queue_input(&mut self){
        let Some(input)=self.restored_queue_input.as_ref() else{return;};
        if !self.composer.text().is_empty()||input["subject_id"].as_str().is_some_and(|s|s!=self.conversation_subject){
            self.notice="Input withdrawn and retained; clear the draft in its topic, then F7 and E to restore it".into();self.mark();return;
        }
        let text=input["text"].as_str().unwrap_or("");
        self.composer=Composer::from_draft(Draft{text:text.into(),row:text.split('\n').count()-1,col:text.rsplit('\n').next().unwrap_or("").chars().count(),literal_paste:true,..Draft::default()});
        self.restored_queue_input=None;self.input_history.reset_navigation();self.modal=Modal::None;self.focus=Focus::Composer;
        self.discard_reply_context();self.notice="Input restored for editing; Enter steers, Alt+Enter queues a follow-up".into();self.mark();
    }
    fn foreground_coordinator(&self)->Option<&Value>{
        let c=self.live.as_ref()?.conversation.as_ref()?;
        let rows=if self.continuous_mode(){c["continuous"]["queue"].as_array()?}else{c["turns"].as_array()?};
        rows.iter().find(|r|r["status"]=="reserved"&&(!self.coordination_mode()||r["subject_id"]==self.conversation_subject))
    }
    fn live_steering_available(&self)->bool{self.live.as_ref().and_then(|l|l.conversation.as_ref()).is_some_and(|c|c["continuous"]["live_input_contract"]==1)}
    fn interrupt_coordinator(&mut self){
        let Some(turn)=self.foreground_coordinator().and_then(|r|r["turn_id"].as_str()).map(str::to_owned) else{
            self.notice="No foreground coordinator call is active".into();self.mark();return;
        };
        if self.foreground_coordinator().is_some_and(|r|r["interrupt_requested"]==true) || self.interrupt_pending.as_ref().is_some_and(|(t,_)|t==&turn){
            self.notice="Stopping coordinator; you can keep drafting or leave".into();self.mark();return;
        }
        self.interrupt_pending=Some((turn.clone(),None));
        self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"cancel_conversation","payload":{"turn_id":turn}})));
        self.notice="Requesting coordinator interruption; you can keep drafting or leave".into();self.mark();
    }
    pub fn interrupt_command_started(&mut self,action:&str,id:&str){
        if action=="cancel_conversation"&&let Some((_,command))=&mut self.interrupt_pending{*command=Some(id.into());}
    }
    pub fn interrupt_command_result(&mut self,id:&str,outcome:&Value){
        if self.interrupt_pending.as_ref().is_some_and(|(_,command)|command.as_deref()==Some(id))&&outcome["status"]!="accepted"{self.interrupt_pending=None;}
    }
    pub fn interrupt_not_sent(&mut self){self.interrupt_pending=None;}
    fn detach_now(&mut self){self.intent(LiveIntent::Detach);self.detached=true;self.mark();}
    fn open_question_suggestion(&mut self){self.question_confirmation=self.question_suggestion().cloned();if let Some(a)=self.question_confirmation.as_ref(){self.scope_editor=Composer::from_draft(Draft{text:a["body"]["action"]["payload"]["answer"].as_str().unwrap_or("").into(),multiline:true,..Draft::default()});}self.open(Modal::QuestionConfirm);}
    fn native_dialogue_blocked(&self)->bool{self.continuous_mode()&&self.chat().is_some_and(|v|v["profile"]["mode"]=="native"&&v["status"]=="blocked")}
    fn confirmation_controls(&self)->Vec<&'static str>{if !self.confirmation_current(){vec!["Back"]}else if self.question_confirmation.as_ref().is_some_and(|v|v["body"]["action"]["payload"]["disposition"]=="defer"){vec!["Back","Record formal deferral"]}else{vec!["Back","Edit answer","Record formal answer"]}}
    fn can_release_held(&self)->bool{self.dialogue_state()["pause"]["hard"]!=true&&self.chat().is_some_and(|v|matches!(v["status"].as_str(),Some("recoverable"|"available"|"running")))}
    fn confirmation_current(&self)->bool {self.question_confirmation.as_ref().is_some_and(|a|self.question_suggestion()==Some(a))}
    pub fn conversation_ui_state(&self)->Value{
        let mut drafts=self.journal.drafts.iter().filter(|(k,_)|k.starts_with("subject:")).take(16).map(|(k,v)|(k.clone(),serde_json::to_value(v).unwrap_or(Value::Null))).collect::<serde_json::Map<_,_>>();
        drafts.insert(format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target)),serde_json::to_value(self.input_history.draft_to_save(self.composer.snapshot())).unwrap_or(Value::Null));
        let mut contexts=self.draft_replies.iter().take(16).map(|(k,v)|(k.clone(),v.clone())).collect::<serde_json::Map<_,_>>();let current=if self.input_history.is_recalling(){self.history_reply.clone()}else{self.draft_reply.clone()};if let Some(context)=current{contexts.insert(format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target)),context);}contexts.retain(|key,_|drafts.get(key).and_then(|d|d["text"].as_str()).is_some_and(|text|!text.is_empty()));
        serde_json::json!({"schema_version":2,"subject_id":self.conversation_subject,"request_revision":self.conversation_revision_request,"drafts":drafts,"draft_contexts":contexts,"restored_input":self.restored_queue_input,"last_queue_restore":self.last_queue_restore})
    }
    pub fn restore_conversation_ui(&mut self,v:&Value)->Result<(),String>{
        let id=v["subject_id"].as_str().ok_or("Saved conversation subject is missing")?;
        if !matches!(v["schema_version"].as_u64(),Some(1|2))||id.is_empty()||id.len()>128||!id.bytes().all(|c|c.is_ascii_alphanumeric()||b"-_".contains(&c)){return Err("Saved conversation subject is invalid".into())}
        if v["schema_version"]==1&&self.live.as_ref().and_then(|l|l.conversation.as_ref()).and_then(|c|c["coordination"]["resume_subject"].as_str()).is_some_and(|hint|hint!=id){self.journal.drafts.insert(format!("subject:{id}:{}",Self::request_draft_slot(self.journal.target)),self.composer.snapshot());self.composer=Composer::default();}
        self.last_queue_restore=v["last_queue_restore"].as_str().filter(|s|s.len()<=128).map(str::to_owned);
        if let Some(input)=v.get("restored_input").filter(|v|!v.is_null()){
            if input["text"].as_str().is_none_or(|t|t.len()>32768){return Err("Restored input exceeds bounds".into())}self.restored_queue_input=Some(input.clone());
        }
        self.conversation_subject=if v["schema_version"]==1{self.live.as_ref().and_then(|l|l.conversation.as_ref()).and_then(|c|c["coordination"]["resume_subject"].as_str()).unwrap_or(id).into()}else{id.into()};self.conversation_revision_request=v["request_revision"]==true;
        if let Some(drafts)=v["drafts"].as_object(){if drafts.len()>17{return Err("Too many saved request drafts".into())}for(k,value)in drafts{let draft:Draft=serde_json::from_value(value.clone()).map_err(|e|e.to_string())?;if !k.starts_with("subject:")||k.len()>200||draft.text.len()>32768{return Err("Saved request draft exceeds bounds".into())}self.journal.drafts.insert(k.clone(),draft);}let key=format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target));if let Some(draft)=self.journal.drafts.get(&key){self.composer=Composer::from_draft(draft.clone());}}
        if let Some(contexts)=v["draft_contexts"].as_object(){if contexts.len()>17{return Err("Too many saved reply contexts".into())}for(k,c)in contexts{if k.len()>200||!k.starts_with("subject:")||waywr_backend::codec::canonical(c).map_err(|e|e.to_string())?.len()>8192{return Err("Invalid saved reply context".into())}if self.journal.drafts.get(k).is_some_and(|d|!d.text.is_empty())||k==&format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target))&&!self.composer.text().is_empty(){self.draft_replies.insert(k.clone(),c.clone());}}self.draft_reply=self.draft_replies.get(&format!("subject:{}:{}",self.conversation_subject,Self::request_draft_slot(self.journal.target))).cloned();}
        if self.composer.text().is_empty(){self.discard_reply_context();}
        self.freeze_unbound_draft();
        self.select_conversation_scope();Ok(())
    }
    pub fn message_payload(&self,text:&str)->Value{let mut p=serde_json::json!({"text":text,"intent":"discussion","subject_id":self.conversation_subject,"request_revision":self.conversation_revision_request,"acknowledged_outcome":if self.chat().is_some_and(|v|v["status"]=="recoverable"){self.dialogue_state()["pause"].clone()}else{Value::Null}});if self.coordination_mode(){p["interaction"]=self.draft_reply.clone().filter(|c|c["subject_id"]==self.conversation_subject).unwrap_or_else(||self.coordination_context("composer"));}p}
    fn dialogue_modal(&self)->Option<(String,String)>{
      if !self.continuous_mode(){return None}
      match self.modal {
       Modal::Queue=>Some(self.queue_body()),
       Modal::QuestionConfirm=>{let Some(a)=self.question_confirmation.as_ref().filter(|_|self.confirmation_current())else{return Some(("Suggestion changed".into(),"No current formal suggestion to record. Close and continue discussing.".into()))};let p=&a["body"]["action"]["payload"];Some(("Confirm formal question decision".into(),format!("Suggested {} for question {}\n\n{}\n\nTab to edit or record. Ordinary discussion needs no confirmation. Recording uses no call; it grants no scope authorization.",p["disposition"].as_str().unwrap_or("answer"),p["question_id"].as_str().unwrap_or(""),self.scope_editor.text())))},
       Modal::Help=>Some(("Continuous conversation".into(),"Send discussion while a question or scope is pending. Enter on empty input opens the visible action; while running it interrupts the foreground coordinator directly. Keep composing. F7 shows queued/held messages with explicit release/withdraw. /plan opens optional scope review; only its final Authorize scope action grants permission. Suggested formal answers/deferrals require Record confirmation; normal chat needs none. New Send after a proven recoverable failure requests only that new turn, leaving older messages held. /detach preserves supervision; /exit leaves immediately; /pause stops dispatch; resume reconciles. Mock transport is scripted regression data, not a live coordinator.".into())),
       Modal::Settings=>Some(("Coordinator settings · continuous scope".into(),format!("Transport: {}\nClaude Code · claude-sonnet-5-5 · medium\nReadiness: {}\n{}\n\nAt most six coordinator invocations. Zero workers, consultation or execution admissions; no retry, repair or fallback. Bounded project context; no inherited hooks/history/MCP or filesystem/process tools. Only inert StructuredOutput is enabled. Native allowance requires separate authorization. Mock transport is scripted regression data.\n\nTheme {} (t) · Effects {} (e)",if self.chat()?["profile"]["mode"]=="mock-cli"{"MOCK CLI STREAM"}else{"Native · separately authorized"},self.chat()?["status"].as_str().unwrap_or("blocked"),self.chat()?["reason"].as_str().unwrap_or(""),self.journal.theme,self.journal.effects))),
       Modal::Inspect=>Some(("Coordinator conversation · project evidence".into(),format!("Provider/model: Claude / {}\nReadiness: {}\nConversation allowance: {}/{} charged\n\nSaved scope (if selected):\n{}\n\nQueue/recovery:\n{}\n\nTurns and immutable execution-record paths:\n{}\n\nTyped completion, receipt, packet, raw events and reported usage remain in each attempt. Application records determine saves and authorization.",self.chat()?["profile"]["model"],self.chat()?["status"],self.live.as_ref()?.conversation.as_ref()?["budgets"]["conversation"]["charged"],self.live.as_ref()?.conversation.as_ref()?["budgets"]["conversation"]["limit"],serde_json::to_string_pretty(&self.current_scope()).unwrap_or_default(),serde_json::to_string_pretty(&self.dialogue_state()).unwrap_or_default(),serde_json::to_string_pretty(&self.live.as_ref()?.conversation.as_ref()?["turns"]).unwrap_or_default()))),
       _=>None
      }
    }
    pub fn saved_scope_answer(&self)->Option<&Value>{
        if !self.scope_mode()||self.continuous_mode(){return None}self.live.as_ref()?.conversation.as_ref()?["questions"].as_array()?.iter().find(|q|q["status"]=="answered"&&q["scope"]["continuation_available"]==true)
    }
    pub fn scope_mode(&self)->bool{self.live.as_ref().and_then(|v|v.conversation.as_ref()).is_some_and(|v|v["scope_enabled"]==true)}
    pub fn current_scope(&self)->Option<&Value>{
        if self.coordination_mode(){return self.coordination_request()?.get("scope").filter(|v|!v.is_null())}
        let live=self.live.as_ref()?;let row=live.selected(self.journal.selected_task)?;let detail=live.detail.as_ref()?;
        if self.continuous_mode()&&self.conversation_scope_task_id()!=row["task_id"].as_str(){return None}
        (detail["task"]["task_id"]==row["task_id"]&&detail["task"]["version"]==row["version"]&&detail["task"]["state_revision"]==row["state_revision"]&&!detail["scope"].is_null()).then_some(&detail["scope"])
    }
    fn scope_identity(v:&Value)->Value{serde_json::json!({"base":v["base"],"decision_revision":v["decision_revision"],"authorization_command_id":v["authorization_command_id"],"operating_policy":v["operating_policy"]})}
    fn scope_dirty(&self)->bool{self.scope_edit.as_ref().is_some_and(|edit|self.current_scope().is_none_or(|head|edit["payload"]["base"]!=head["base"]||edit["payload"]["proposal"]!=head["record"]["proposal"]))}
    fn scope_unavailable(&self)->Option<&'static str>{
        let Some(head)=self.current_scope()else{return Some("Saved revision is loading")};
        if head["authorized"]==true{return Some("This exact scope is already authorized")}
        if head["needs_revision"]==true{return Some("Linked answers changed; save or request a scope revision")}
        if head["pending_turns"]!=0{return Some("Coordinator changes are pending")}
        if head["questions"].as_array().is_some_and(|a|a.iter().any(|q|q["status"]=="open")){return Some("Answer the linked question first")}
        if !serde_json::from_value::<waywr_backend::scope_contract::Proposal>(head["record"]["proposal"].clone()).is_ok_and(|p|p.eligible()){return Some("Complete the objective, deliverables, scope and acceptance criteria")}
        if self.scope_dirty(){return Some("Save or discard the retained local edit")}
        if !self.continuous_mode()&&self.live.as_ref().and_then(|v|v.conversation.as_ref()).is_none_or(|v|v["turns"].as_array().is_some_and(|a|a.iter().any(|t|t["status"]=="queued"||t["status"]=="reserved"))){return Some("Wait for the pending coordinator reply")}
        None
    }
    fn scope_ready(&self)->bool{self.scope_unavailable().is_none()}
    fn scope_actions(&self)->Vec<(ScopeAction,&'static str)>{
        let Some(head)=self.current_scope()else{return vec![(ScopeAction::Close,"Close")]};
        if head["authorized"]==true {
            return vec![(ScopeAction::Close,"Close"),(ScopeAction::Edit,"Edit scope"),
                (ScopeAction::RequestRevision,"Request revision · 1 call")];
        }
        let primary=match self.scope_step {
            0=>(ScopeAction::Next,"Continue to boundaries"),
            1=>(ScopeAction::Next,"Continue to choices"),
            2 if head["record"]["proposal"]["unresolved_decisions"].as_array().is_some_and(|v|!v.is_empty())
                =>(ScopeAction::Acknowledge,"Acknowledge choices · continue"),
            2=>(ScopeAction::Acknowledge,"Continue to authorization"),
            _=>(ScopeAction::Authorize,"Authorize scope"),
        };
        let mut actions=vec![primary];
        if self.scope_step>0{actions.push((ScopeAction::Previous,"Previous step"));}
        actions.extend([(ScopeAction::Edit,"Edit scope"),
            (ScopeAction::RequestRevision,"Request revision · 1 call"),(ScopeAction::Close,"Not now")]);
        actions
    }
    fn scope_move(&mut self,step:usize){
        self.scope_step=step.min(3);
        self.modal_action=0;
        self.modal_scroll=0;
        self.modal_scroll_limit=0;
        self.notice=match self.scope_step {
            0=>"Review purpose and deliverables; Continue does not authorize",
            1=>"Review boundaries and acceptance; Continue does not authorize",
            2=>"Review unresolved choices; acknowledgement does not authorize",
            _=>"Final step: only Authorize scope saves permission; execution stays unavailable",
        }.into();
    }
    fn scope_action(&mut self){
        let Some((action,_))=self.scope_actions().get(self.modal_action).copied()else{return};
        match action {
            ScopeAction::Close=>self.modal=Modal::None,
            ScopeAction::Next=>self.scope_move(self.scope_step+1),
            ScopeAction::Previous=>self.scope_move(self.scope_step.saturating_sub(1)),
            ScopeAction::Edit=>self.scope_start_edit(),
            ScopeAction::RequestRevision=>{
                self.set_target(Target::Coordinator);self.conversation_revision_request=self.continuous_mode();self.modal=Modal::None;
                self.focus=Focus::Composer;
                self.notice="Describe the revision; sending uses a remaining coordinator call".into();
            },
            ScopeAction::Acknowledge=>{
                if let Some(head)=self.current_scope(){
                    self.scope_ack=waywr_backend::codec::canonical_sha256(&head["record"]["proposal"]["unresolved_decisions"]).ok();
                    self.scope_move(3);
                }
            },
            ScopeAction::Authorize=>{
                if !self.scope_ready(){self.notice=self.scope_unavailable().unwrap_or("Scope is not ready").into();return}
                let head=self.current_scope().unwrap();
                let hash=waywr_backend::codec::canonical_sha256(&head["record"]["proposal"]["unresolved_decisions"]).unwrap_or_default();
                if self.scope_step!=3||self.scope_ack.as_deref()!=Some(&hash){
                    self.scope_move(2);return;
                }
                if self.coordination_mode(){self.submit_inline_approval("scope_details");self.modal=Modal::None;return;}
                let mut payload=serde_json::json!({"base":head["base"],"decision_revision":head["decision_revision"],
                    "authorization_command_id":head["authorization_command_id"],"acknowledged_unresolved_sha256":hash});
                if self.development_mode(){payload["operating_policy_sha256"]=serde_json::json!(waywr_backend::codec::canonical_sha256(&head["operating_policy"]).unwrap_or_default());}
                self.intent(LiveIntent::AuthorizeScope(payload));self.modal=Modal::None;
                self.notice=if self.development_mode(){"Scope and policy decision pending durable acceptance"}else{"Scope decision pending durable acceptance; execution unavailable"}.into();
            },
        }
        self.mark();
    }
    fn scope_fields(body:&mut String,proposal:&Value,fields:&[(&str,&str)]){
        for (label,name) in fields {
            body.push_str(&format!("\n**{label}**\n"));
            let items=proposal[*name].as_array();
            if items.is_none_or(Vec::is_empty){body.push_str("  None recorded\n");}
            if let Some(items)=items{for item in items{body.push_str(&format!("  • {}\n",item.as_str().unwrap_or("")));}}
        }
    }
    pub fn development_mode(&self)->bool{self.live.as_ref().and_then(|v|v.conversation.as_ref()).is_some_and(|v|!v["development"].is_null())}
    pub fn development_workflow(&self)->Option<Value>{let id=if self.coordination_mode(){self.conversation_scope_task_id()?}else{self.live.as_ref()?.selected(self.journal.selected_task)?["task_id"].as_str()?};self.development_view()["workflows"].as_array()?.iter().find(|w|w["work_id"]==id).cloned()}
    pub fn development_steering_payload(&self,text:&str)->Option<Value>{let selected=self.live.as_ref()?.selected(self.journal.selected_task)?;let dev=self.development_view();let work=dev["workflows"].as_array()?.iter().find(|w|w["work_id"]==selected["task_id"])?;Some(serde_json::json!({"work_id":work["work_id"],"requirement_generation":work["requirement_generation"],"in_scope":true,"text":text}))}
    pub fn development_reply_payload(&self,text:&str)->Option<Value>{let work=self.development_workflow()?;if work["status"]!="attention"||work["question"].is_null(){return None}let mut p=serde_json::json!({"work_id":work["work_id"],"revision":work["revision"],"text":text});if self.coordination_mode(){let question=self.draft_reply.as_ref().and_then(|c|c.get("development_question")).filter(|q|!q.is_null()).cloned().unwrap_or_else(||serde_json::json!({"work_id":work["work_id"],"revision":work["revision"],"question_sha256":waywr_backend::codec::canonical_sha256(&work["question"]).unwrap_or_default()}));p["revision"]=question["revision"].clone();p["question_sha256"]=question["question_sha256"].clone();p["coordination_version"]=serde_json::json!(1);}Some(p)}
    pub fn development_resume_payload(&self)->Option<Value>{
        let work=self.development_workflow()?;
        if work["status"]!="attention" || !work["question"].is_null() || !work["pending"].is_null(){return None}
        Some(serde_json::json!({"work_id":work["work_id"],"revision":work["revision"]}))
    }
    fn development_view(&self)->&Value{self.live.as_ref().and_then(|v|v.conversation.as_ref()).map_or(&Value::Null,|v|&v["development"])}
    fn scope_body(&self)->(String,String){
        let Some(head)=self.current_scope()else{return ("Scope loading".into(),"Waiting for the saved revision; no decision is available.".into())};
        let p=&head["record"]["proposal"];
        let mut body=format!("{} · revision {}\n",p["title"].as_str().unwrap_or("Scope"),head["base"]["revision"]);
        if head["authorized"]==true {
            body.push_str(if self.development_mode(){"Scope and operating policy authorized · development enabled\n\n**Objective**\n"}else{"Scope authorized · execution unavailable\n\n**Objective**\n"});
            body.push_str(p["objective"].as_str().unwrap_or(""));body.push('\n');
            Self::scope_fields(&mut body,p,&[("Deliverables","deliverables"),("In scope","in_scope"),
                ("Out of scope","out_of_scope"),("Constraints","constraints"),
                ("Acceptance criteria","acceptance_criteria"),("Unresolved decisions","unresolved_decisions")]);
            body.push_str(if self.development_mode(){"\nApproved work proceeds automatically. See F6 for assignments and /inspect for exact commits, checks and request history."}else{"\nThis saved revision has permission for its scope only. No workers launched.\nEditing or refining creates a new revision that needs a separate decision."});
            return ("View authorized scope".into(),body);
        }
        let title=match self.scope_step {
            0=>{
                body.push_str("\n**Objective**\n");body.push_str(p["objective"].as_str().unwrap_or(""));body.push('\n');
                Self::scope_fields(&mut body,p,&[("Deliverables","deliverables")]);
                body.push_str("\nContinue when these describe your goal. No permission is saved at this step.");
                "Step 1 of 4 · Purpose and deliverables"
            },
            1=>{
                Self::scope_fields(&mut body,p,&[("In scope","in_scope"),("Out of scope","out_of_scope"),
                    ("Constraints","constraints"),("Acceptance criteria","acceptance_criteria")]);
                if self.development_mode(){body.push_str(&format!("\n**Operating policy included in this approval**\n{}\n",serde_json::to_string_pretty(&head["operating_policy"]).unwrap_or_default()));}
                body.push_str("\nContext: selected project discussion and linked answers only. Inspect shows provenance.\nContinue when these boundaries are right; no permission is saved.");
                "Step 2 of 4 · Boundaries and acceptance"
            },
            2=>{
                Self::scope_fields(&mut body,p,&[("Unresolved decisions","unresolved_decisions")]);
                if let Some(questions)=head["questions"].as_array().filter(|v|!v.is_empty()) {
                    body.push_str("\n**Linked answers**\n");
                    for q in questions{body.push_str(&format!("  • {}: {}\n",q["prompt"].as_str().unwrap_or("Question"),q["answer"].as_str().unwrap_or("Answer still needed")));}
                }
                body.push_str("\nAcknowledging accepts that the listed choices remain unresolved. It does not authorize scope.\nUse Edit scope to resolve them; otherwise acknowledge and continue to the final decision.");
                "Step 3 of 4 · Unresolved choices"
            },
            _=>{
                if self.development_mode(){body.push_str("\n**Final decision**\nAuthorize this exact saved scope and the operating policy shown in Boundaries. Waywr will launch Codex workers, run checks, commit, integrate and advance clean local main automatically within that scope. No mandatory call/time/repair ceiling. Capacity waits preserve work; provider fallback is disabled. Remote publication remains outside this approval.\n");}else{
                body.push_str("\n**Final decision**\nAuthorize this exact saved revision, including the choices acknowledged in the previous step.\n\nAfter authorization: Scope authorized · execution unavailable\nNo execution allowance, workers or consultation follow. This action uses no model call.\n\nPrevious step reviews your choices; Not now closes without saving permission.");
                }
                if let Some(reason)=self.scope_unavailable(){body.push_str(&format!("\n\n**Authorization unavailable**\n{reason}."));}
                else{body.push_str("\n\nReady for explicit scope authorization. Only selecting Authorize scope saves it.");}
                "Step 4 of 4 · Explicit authorization"
            },
        };
        (title.into(),body)
    }
    fn scope_open(&mut self){
        if self.current_scope().is_some(){self.open(Modal::Scope)}else if let Some(id)=self.live.as_ref().and_then(|v|v.selected(self.journal.selected_task)).and_then(|v|v["task_id"].as_str()).map(str::to_owned){self.pending_primary_task=Some(id);self.intent(LiveIntent::Inspect);self.notice="Loading the saved proposal revision…".into();}
    }
    const SCOPE_FIELDS:[&'static str;9]=["title","objective","deliverables","in_scope","out_of_scope","constraints","acceptance_criteria","unresolved_decisions","rationale"];
    fn scope_store_field(&mut self){
        if let Some(edit)=&mut self.scope_edit{let name=Self::SCOPE_FIELDS[self.scope_field];let text=self.scope_editor.text();
            if name=="rationale"{edit["payload"][name]=Value::String(text)}else if matches!(name,"title"|"objective"){edit["payload"]["proposal"][name]=Value::String(text)}else{edit["payload"]["proposal"][name]=serde_json::json!(text.lines().filter(|v|!v.trim().is_empty()).collect::<Vec<_>>())}
        }
    }
    fn scope_load_field(&mut self){if let Some(edit)=&self.scope_edit{let name=Self::SCOPE_FIELDS[self.scope_field];let v=if name=="rationale"{&edit["payload"][name]}else{&edit["payload"]["proposal"][name]};let text=if let Some(s)=v.as_str(){s.to_owned()}else{v.as_array().map(|a|a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n")).unwrap_or_default()};self.scope_editor=Composer::from_draft(Draft{text,multiline:true,..Draft::default()});}}
    fn scope_start_edit(&mut self){
        if self.scope_edit.is_none(){let Some(h)=self.current_scope().cloned()else{return};self.scope_edit=Some(serde_json::json!({"schema_version":1,"payload":{"base":h["base"],"decision_revision":h["decision_revision"],"authorization_command_id":h["authorization_command_id"],"proposal":h["record"]["proposal"],"rationale":"Local scope edit"}}));self.scope_field=1;self.scope_load_field();}
        self.open(Modal::ScopeEditor);self.modal_action=1;
    }
    pub fn scope_edit_snapshot(&mut self)->Value{self.scope_store_field();if let Some(edit)=&mut self.scope_edit{edit["field"]=serde_json::json!(self.scope_field);edit["draft"]=serde_json::to_value(self.scope_editor.snapshot()).unwrap_or(Value::Null);}self.scope_edit.clone().unwrap_or(Value::Null)}
    pub fn restore_scope_edit(&mut self,edit:Value){
        if edit.is_null(){return}if edit["schema_version"]!=1||serde_json::from_value::<waywr_backend::scope_contract::Proposal>(edit["payload"]["proposal"].clone()).is_err(){self.notice="Saved scope edit requires inspection; not applied".into();return}
        self.scope_field=edit["field"].as_u64().unwrap_or(1).min(8) as usize;self.scope_edit=Some(edit.clone());self.scope_editor=Composer::from_draft(serde_json::from_value::<Draft>(edit["draft"].clone()).unwrap_or_default());
    }
    fn scope_editor_input(&mut self,event:Event){
        if self.scope_edit.as_ref().is_some_and(|e|e.get("pending_command_id").is_some()){if matches!(event,Event::Key(KeyEvent{code:KeyCode::Esc,..})){self.open(Modal::Scope);}self.notice="Save delivery pending; draft retained for reconciliation".into();return}
        if let Event::Key(k)=event{if k.kind==KeyEventKind::Release{return}
            if k.code==KeyCode::Esc{self.scope_store_field();self.open(Modal::Scope);return}
            if matches!(k.code,KeyCode::Tab|KeyCode::BackTab)||(self.modal_action!=1&&matches!(k.code,KeyCode::Left|KeyCode::Right)){self.scope_store_field();self.modal_action=if matches!(k.code,KeyCode::Tab|KeyCode::Right){(self.modal_action+1)%5}else{(self.modal_action+4)%5};self.mark();return}
            if self.modal_action==0&&matches!(k.code,KeyCode::Up|KeyCode::Down){self.scope_store_field();self.scope_field=if k.code==KeyCode::Down{(self.scope_field+1)%9}else{(self.scope_field+8)%9};self.scope_load_field();self.mark();return}
            if k.code==KeyCode::Enter&&self.modal_action!=1{match self.modal_action{0=>self.modal_action=1,2=>{self.scope_store_field();if let Some(edit)=&self.scope_edit{let payload=edit["payload"].clone();if serde_json::from_value::<waywr_backend::scope_contract::Proposal>(payload["proposal"].clone()).is_ok_and(|p|p.validate().is_ok())&&payload["rationale"].as_str().is_some_and(|v|!v.trim().is_empty()){self.intent(LiveIntent::SaveScope(payload));self.open(Modal::Scope);self.notice="Revision pending durable acceptance; edit retained".into();}else{self.notice="Correct the proposal field bounds/rationale before saving".into();}}},3=>self.open(Modal::Scope),4=>{self.scope_edit=None;self.open(Modal::Scope);},_=>{}}self.mark();return}
        }
        if self.modal_action==1{let before=self.scope_editor.snapshot();if let EditResult::Refused(reason)=self.scope_editor.input(event){self.notice=reason.into();}self.scope_store_field();let name=Self::SCOPE_FIELDS[self.scope_field];let text=self.scope_editor.text();let valid=if name=="rationale"{text.len()<=4096}else{text.len()<=if name=="title"{480}else if name=="objective"{4096}else{16384}&&self.scope_edit.as_ref().is_none_or(|e|waywr_backend::codec::canonical(&e["payload"]["proposal"]).is_ok_and(|b|b.len()<=16384))};if !valid{self.scope_editor=Composer::from_draft(before);self.scope_store_field();self.notice="Edit exceeds the field/proposal limit; previous draft retained".into();}self.mark();}
    }
    fn pending_consultation(&self)->Option<&Value>{
        self.live.as_ref()?.conversation.as_ref()?["actions"].as_array()?
            .iter().find(|action|action["kind"]=="request_consultation")
    }
    fn pending_steering(&self)->Option<&Value>{
        let live=self.live.as_ref()?;
        let selected=live.selected(self.journal.selected_task)?["task_id"].as_str()?;
        live.conversation.as_ref()?["actions"].as_array()?.iter()
            .find(|action|action["kind"]=="propose_steering" &&
                action["body"]["payload"]["task_id"]==selected)
    }
    fn candidate_read(&self) -> bool {
        if let Some(live) = &self.live {
            let Some(selected) = live.selected(self.journal.selected_task) else { return false; };
            let Some(detail) = &live.detail else { return false; };
            let identity = Self::candidate_identity(&live.project, detail);
            return selected["status"] == "awaiting-input"
                && detail["task"]["task_id"] == selected["task_id"]
                && detail["task"]["version"] == selected["version"]
                && detail["task"]["state_revision"] == selected["state_revision"]
                && live.artifact_complete
                && identity.is_some()
                && live.reviewed_identity == identity;
        }
        self.journal.first == First::Review
            && self.journal.candidate_sha256.is_some()
            && self.journal.reviewed_sha256 == self.journal.candidate_sha256
    }
    fn primary(&mut self) {
        if self.foreground_coordinator().is_some(){self.interrupt_coordinator();return;}
        if self.model_setup_needed(){self.open_model_setup();return;}
        if self.coordination_mode()&&self.focus==Focus::Activity&&self.coordination_card().is_some(){self.inline_card_action();return;}
        if self.coordination_mode()&&self.focus==Focus::Activity&&self.question_suggestion().is_some()&&self.card_action==1{self.confirm_inline_answer();return;}
        if self.continuous_mode(){let rows=self.queue_rows();if let Some(r)=rows.iter().find(|r|r["status"]=="reserved"&&(!self.coordination_mode()||r["subject_id"]==self.conversation_subject)){self.open(if r["interrupt_requested"]==true{Modal::Queue}else{Modal::Cancel});return}if self.question_suggestion().is_some(){self.open_question_suggestion();return}if rows.iter().any(|r|r["status"]=="held"){self.open(Modal::Queue);return}}
        if self.chat().is_some_and(|v|v["status"]=="awaiting-authorization"){self.open(Modal::Settings);return;}
        if self.pending_conversation_question().is_some()||self.saved_scope_answer().is_some(){self.open(Modal::Question);return;}
        if self.development_mode()&&self.current_scope().is_some_and(|s|s["authorized"]==true){self.open(Modal::Inspect);return;}
        if self.scope_mode()&&self.live.as_ref().is_some_and(|v|!v.tasks.is_empty()){self.scope_open();return;}
        if self.pending_consultation().is_some(){self.open(Modal::Consultation);return;}
        if self.pending_steering().is_some(){self.open(Modal::Steering);return;}
        if self.live.as_ref().is_some_and(|live|live.tasks.is_empty() && live.conversation.is_some()){
            self.open(Modal::Inspect);return;
        }
        if let Some(live) = &self.live {
            let status = live.status(self.journal.selected_task).to_owned();
            let selected = live
                .selected(self.journal.selected_task)
                .and_then(|v| v["task_id"].as_str())
                .map(str::to_owned);
            if status == "proposed" {
                if live
                    .plan
                    .as_ref()
                    .is_some_and(|v| v["plan"]["task_id"].as_str() == selected.as_deref())
                {
                    self.open(Modal::Scope);
                } else {
                    self.intent(LiveIntent::Plan);
                    self.notice = "Loading scope for review…".into();
                }
            } else if status == "awaiting-input" {
                let current = live.detail.as_ref().is_some_and(|v| {
                    v["task"]["task_id"].as_str() == selected.as_deref()
                        && v["task"]["version"]
                            == live
                                .selected(self.journal.selected_task)
                                .map_or(Value::Null, |row| row["version"].clone())
                        && v["task"]["state_revision"]
                            == live
                                .selected(self.journal.selected_task)
                                .map_or(Value::Null, |row| row["state_revision"].clone())
                });
                if current {
                    self.open_selected_action();
                } else if let Some(id) = selected {
                    self.pending_primary_task = Some(id);
                    self.intent(LiveIntent::Inspect);
                    self.notice = "Loading this task's next action…".into();
                }
            } else {
                self.open(Modal::Inspect);
                self.intent(LiveIntent::Inspect);
            }
            return;
        }
        if self.journal.selected_task == 0 {
            match self.journal.first {
                First::Suggested => {
                    self.journal.propose_first();
                    self.open(Modal::Scope);
                }
                First::Proposed => self.open(Modal::Scope),
                First::Question => self.open(Modal::Question),
                First::Review => self.open(Modal::Review),
                First::Drafting | First::Critiquing | First::Revising => self.open(Modal::Worker),
                _ => self.open(Modal::Inspect),
            }
        } else if self.journal.second == Second::Proposed {
            self.open(Modal::Scope)
        } else if self.journal.second == Second::None {
            self.notice = "No second task yet; choose New task".into();
        } else {
            self.open(Modal::Inspect)
        }
        self.mark();
    }
    fn open_selected_action(&mut self) {
        if self.scope_mode(){self.open(Modal::Scope);return;}
        let Some(live) = &self.live else {
            return;
        };
        let selected = live
            .selected(self.journal.selected_task)
            .and_then(|v| v["task_id"].as_str());
        let pending = live
            .detail
            .as_ref()
            .filter(|detail| detail["task"]["task_id"].as_str() == selected)
            .map(|detail| &detail["pending"]);
        match pending.and_then(|value| value["kind"].as_str()) {
            Some("choice") => self.open(Modal::Question),
            Some("final-review") => {
                self.open(Modal::Review);
                self.modal_action = 1;
            }
            _ => self.open(Modal::Inspect),
        }
    }
    fn enabled_reason(&self, name: &str) -> Option<&'static str> {
        if name=="resume" {return if self.live.as_ref().is_some_and(|l|l.phase=="paused") || self.development_resume_payload().is_some(){None}else if self.development_reply_payload("").is_some(){Some("Session is running; answer the saved development question with /reply")}else{Some("Session is already running; no selected workflow awaits operational recovery")}}
        if name=="reply" {return if self.development_reply_payload("").is_some(){None}else{Some("No current development question; use coordinator conversation")}}
        if self.development_mode() && name=="steer"{return if self.development_steering_payload("").is_some(){None}else{Some("Authorize a development scope first")}}
        if self.scope_mode(){return match name{
            "consult"|"open"|"approve"|"reject"|"steer"|"herdr"|"include"|"revoke"=>Some("Scope only: execution, artifact approval and consultation are unavailable"),
            "queue" if !self.continuous_mode()=>Some("Queue inspection requires the continuous conversation release"),
            "answer" if self.question_suggestion().is_none()&&self.pending_conversation_question().is_none()&&self.saved_scope_answer().is_none()=>Some("No question or saved continuation is pending"),
            "plan"|"confirm" if self.live.as_ref().is_none_or(|v|v.tasks.is_empty())=>Some("No saved proposal yet; discuss it with the coordinator"),_=>None};}
        if self.chat().is_some() && matches!(name,"new"|"plan"|"confirm"|"answer"|"consult"|"open"|"approve"|"reject"|"steer"|"herdr"|"include"|"revoke"){
            return Some("Conversation only: worker execution, consultation and steering are deferred");
        }
        if let Some(live)=&self.live {
            let status=live.status(self.journal.selected_task);
            return match name {
                "answer" if self.pending_conversation_question().is_none() && status!="awaiting-input" => Some("no pending question in this project"),
                "answer" if self.pending_conversation_question().is_none() && live.detail.is_some() && self.pending_choice().is_none() => Some("selected task has no current choice question"),
                "consult" if self.pending_consultation().is_none()=>Some("no consultation request is pending"),
                "approve" | "reject" | "open" if status!="awaiting-input" => Some("selected task has no final review"),
                "plan" | "confirm" if status!="proposed" => Some("selected task has no proposed scope"),
                "steer" if self.pending_steering().is_none() && status!="proposed"=>
                    Some("select an unadmitted proposed task before suggesting changes"),
                "herdr" if live.operations.is_empty() => Some("open /output and select an operation first"),
                "include" | "revoke" => Some("use the explicit project context CLI until this view is connected"),
                _ => None,
            };
        }
        if name=="pause" { return Some("session pause is available in the installed application"); }
        match name {
            "answer" if self.journal.first != First::Question => Some("no pending question"),
            "approve" | "reject" | "open" if self.journal.first != First::Review => {
                Some("no final candidate")
            }
            "plan" | "confirm"
                if (self.journal.selected_task == 0 && !matches!(self.journal.first, First::Suggested | First::Proposed))
                    || (self.journal.selected_task == 1
                        && self.journal.second != Second::Proposed) =>
            {
                Some("selected task has no proposed scope")
            }
            "steer" => Some("steering is not available in this demo"),
            "herdr" => Some("optional views require an installed project operation"),
            "include" | "revoke" => Some("no selected import in this demo"),
            _ => None,
        }
    }
    fn matches(&self) -> Vec<Command> {
        let text = self.composer.text();
        let query = text.strip_prefix('/').unwrap_or("").to_ascii_lowercase();
        let mut list: Vec<Command> = COMMANDS
            .iter()
            .copied()
            .filter(|c| c.name.starts_with(&query))
            .collect();
        list.extend(COMMANDS.iter().copied().filter(|c| {
            !c.name.starts_with(&query) && c.description.to_ascii_lowercase().contains(&query)
        }));
        list
    }
    fn command(&mut self, name: &str) {
        let mut draft = self.composer.snapshot();
        if draft.text.split_whitespace().next() != Some(format!("/{name}").as_str()) {
            draft = Draft { text: format!("/{name}"), col: name.len() + 1, ..Draft::default() };
        }
        self.input_history.remember(draft);
        if matches!(name,"detach"|"exit"|"quit"){
            self.composer.clear();self.discard_reply_context();self.menu=false;self.detach_now();return;
        }
        if name=="interrupt"{
            self.composer.clear();self.discard_reply_context();self.menu=false;self.interrupt_coordinator();return;
        }
        if let Some(reason) = self.enabled_reason(name) {
            self.notice = format!("/{name} unavailable: {reason}");
            self.mark();
            return;
        }
        if self.live.is_some() {
            match name {
                "model"|"thinking"=>{
                    self.open(Modal::Settings);self.provider_section=0;self.provider_field=if name=="thinking"{2}else{1};
                    let names=self.provider_names();let coordinator=self.provider_draft()["coordinator"].as_str().unwrap_or("").to_owned();
                    self.provider_index=names.iter().position(|n|n==&coordinator).unwrap_or(0);
                    self.open_catalog(if name=="thinking"{CatalogScope::Thinking}else{CatalogScope::Models(None)});
                },
                "status"=>self.open(Modal::Status),
                "help" => self.open(Modal::Help),
                "tasks" => self.open(Modal::Tasks),
                "settings" => self.open(Modal::Settings),
                "new" => self.set_target(Target::NewTask),
                "resume" => {
                    let value=if self.live.as_ref().is_some_and(|live|live.phase=="paused") {
                        serde_json::json!({"action":"resume_session","payload":{}})
                    } else {serde_json::json!({"action":"development.resume","payload":self.development_resume_payload().unwrap()})};
                    self.intent(LiveIntent::Dialogue(value));self.composer.clear();self.discard_reply_context();self.menu=false;self.mark();return;
                },
                "reply" => self.set_target(Target::ReplyFirst),
                "inspect" | "grants" => { self.intent(LiveIntent::Inspect); self.open(Modal::Inspect); },
                "output" => {self.open(Modal::Worker);self.intent(LiveIntent::Output);},
                "herdr" => {self.intent(LiveIntent::HerdrView);self.notice="Opening a read-only Herdr view; worker execution is unchanged".into();},
                "plan" | "confirm" if self.scope_mode()=>{
                    if self.continuous_mode() && !self.select_conversation_scope() {
                        self.notice="This topic has no saved scope yet; discuss it with the coordinator first. F6 retains earlier scopes.".into();
                        self.mark();
                    } else { self.scope_open(); }
                },
                "plan" | "confirm" => {
                    let selected=self.live.as_ref().and_then(|v|v.selected(self.journal.selected_task))
                        .and_then(|v|v["task_id"].as_str());
                    if self.live.as_ref().and_then(|v|v.plan.as_ref())
                        .is_some_and(|v|v["plan"]["task_id"].as_str()==selected) { self.open(Modal::Scope); }
                    else { self.intent(LiveIntent::Plan); self.notice="Loading the current scope…".into(); }
                },
                "queue"=>self.open(Modal::Queue),
                "discuss"=>self.discuss_selected_request(),
                "answer" if self.question_suggestion().is_some()=>self.open_question_suggestion(),
                "answer" => { if self.pending_conversation_question().is_none(){self.intent(LiveIntent::Inspect);} self.open(Modal::Question); },
                "consult" => self.open(Modal::Consultation),
                "steer" => if self.pending_steering().is_some(){self.open(Modal::Steering)}
                    else{self.set_target(Target::SteerFirst)},
                "open" => self.intent(LiveIntent::OpenArtifact),
                "approve" | "reject" => { self.intent(LiveIntent::Inspect); self.open(Modal::Review); },
                "cancel" => self.open(Modal::Cancel),
                "detach" => self.open(Modal::Detach),
                "pause" => self.intent(LiveIntent::Pause),
                _ => self.notice=format!("/{name} is unavailable for this project"),
            }
            self.composer.clear(); self.discard_reply_context(); self.menu=false; self.mark();
            return;
        }
        match name {
            "help" => self.open(Modal::Help),
            "tasks" => self.open(Modal::Tasks),
            "settings" => self.open(Modal::Settings),
            "new" => self.set_target(Target::NewTask),
            "inspect" | "grants" => self.open(Modal::Inspect),
            "output" => self.open(Modal::Worker),
            "plan" | "confirm" if self.journal.selected_task == 0 && self.journal.first == First::Suggested => self.primary(),
            "plan" | "confirm" => self.open(Modal::Scope),
            "answer" => self.open(Modal::Question),
            "open" => {
                self.journal.review();
                self.open(Modal::Artifact)
            }
            "approve" | "reject" => self.open(Modal::Review),
            "cancel" => self.open(Modal::Cancel),
            "detach" => self.open(Modal::Detach),
            _ => self.notice = format!("/{name} is unavailable in this scripted prototype"),
        }
        self.composer.clear();
        self.menu = false;
        self.mark();
    }
    fn submit_composer(&mut self) {self.submit_input(false);}
    fn submit_input(&mut self,follow_up:bool) {
        let text = self.composer.text();
        if text.trim().is_empty() {
            if self.live.is_some() && !self.menu {self.primary();}
            return;
        }
        if text.starts_with('/') && !text.contains('\n') && !self.composer.literal_paste {
            let token = text[1..].split_whitespace().next().unwrap_or("");
            if COMMANDS.iter().any(|c| c.name == token) {
                if token=="open" && self.live.is_some(){
                    let args=text[1+token.len()..].trim();
                    if !args.is_empty(){
                        self.input_history.remember(self.composer.snapshot());
                        if matches!(args,"index.html"|"styles.css"|"app.js"|"README.md"){
                            self.intent(LiveIntent::OpenFile(args.to_owned()));
                            self.composer.clear();self.menu=false;self.mark();
                        }else{self.notice="Choose index.html, styles.css, app.js or README.md".into();self.mark();}
                        return;
                    }
                }
                self.command(token);
                return;
            }
            self.open(Modal::UnknownCommand);
            self.input_history.remember(self.composer.snapshot());
            return;
        }
        self.input_history.remember(self.composer.snapshot());
        if self.live.is_some() {
            let chat_resume=self.coordination_mode()&&matches!(text.trim().to_lowercase().as_str(),"resume this session"|"resume the session"|"resume session");
            if !chat_resume&&self.model_setup_needed(){self.open_model_setup();self.mark();return;}
            if self.development_mode()&&!chat_resume&&self.live.as_ref().is_some_and(|l|l.phase!="running") {self.notice="Session paused. Say resume session or use /resume; your draft is preserved.".into();self.mark();return;}
            if let Some(chat)=self.chat(){
                if !chat_resume&&!matches!(chat["status"].as_str(),Some("available"|"running"|"recoverable")){
                    self.notice=chat["reason"].as_str().filter(|s|!s.is_empty()).unwrap_or("Conversation allowance exhausted; inspect saved responses").into();self.mark();return;
                }
                if !self.development_mode() && self.journal.target!=Target::Coordinator && !(self.scope_mode()&&self.journal.target==Target::NewTask){self.notice="Only coordinator conversation is available in this increment; draft preserved".into();self.mark();return;}
            }
            let conversation=self.live.as_ref().is_some_and(|live|live.conversation.is_some());
            if !conversation && self.journal.target != Target::NewTask {
                self.notice="This bound schema-3 service accepts task requests only. Select New task (F4); your draft is preserved.".into();
                self.mark();return;
            }
            if conversation && matches!(self.journal.target,Target::SteerFirst|Target::SteerSecond) &&
                self.live.as_ref().and_then(|live|live.selected(self.journal.selected_task)).is_none(){
                self.notice="Select a task before steering it; draft preserved".into();self.mark();return;
            }
            self.submission_draft=Some(self.composer.snapshot());
            if !follow_up&&self.live_steering_available()&&matches!(self.journal.target,Target::Coordinator|Target::NewTask)&&let Some(turn)=self.foreground_coordinator().and_then(|r|r["turn_id"].as_str()).map(str::to_owned){
                let mut payload=self.message_payload(&text);payload["turn_id"]=serde_json::json!(turn);
                self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"conversation.steer","payload":payload})));
                self.composer.clear();self.menu=false;self.notice="Steering pending durable delivery · applies at the next agent boundary".into();self.mark();return;
            }
            if self.journal.target==Target::ReplyFirst {
                let Some(payload)=self.development_reply_payload(&text)else{self.notice="Question changed; inspect it before answering. Draft preserved.".into();return};
                self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"development.reply","payload":payload})));self.composer.clear();self.set_target(Target::Coordinator);self.menu=false;self.mark();return;
            }
            self.intent(LiveIntent::SubmitTask(text));
            self.notice=if conversation{"Message pending durable delivery…".into()}
                else{"Task request pending durable delivery…".into()};
            self.composer.clear(); self.menu=false; self.mark();
            return;
        }
        let response = self.journal.submit(text);
        self.notice = if response == "Conversation saved in the isolated demo" {
            String::new()
        } else {
            response.into()
        };
        if !response.contains("preserved") {
            self.composer.clear();
            self.schedule();
        }
        self.mark();
    }
    fn handle_modal(&mut self, key: KeyEvent) {
        if self.provider_settings(){self.provider_key(key);return;}
        if key.code == KeyCode::Esc {
            self.modal = if self.modal == Modal::Artifact {
                self.modal_action=if self.candidate_read(){2}else{1};
                Modal::Review
            } else {
                Modal::None
            };
            self.mark();
            return;
        }
        if key.code == KeyCode::PageUp {
            self.modal_scroll = self.modal_scroll.saturating_sub(if key.modifiers.contains(KeyModifiers::ALT){1}else{8});
            self.mark();
            return;
        }
        if key.code == KeyCode::PageDown {
            self.modal_scroll = self.modal_scroll.saturating_add(if key.modifiers.contains(KeyModifiers::ALT){1}else{8}).min(self.modal_scroll_limit);
            self.mark();
            return;
        }
        if self.modal == Modal::History {
            if matches!(key.code, KeyCode::Up | KeyCode::Down) { self.modal_scroll = 0; }
            match key.code {
                KeyCode::Up => self.history_index = self.history_index.saturating_sub(1),
                KeyCode::Down => self.history_index = (self.history_index + 1).min(self.input_history.len().saturating_sub(1)),
                KeyCode::Enter if key.kind == KeyEventKind::Press => {
                    if let Some(draft) = self.input_history.select_recent(self.history_index, self.composer.snapshot()) {
                        self.restore_recalled_input(draft);
                    }
                    self.modal = Modal::None;
                    self.focus = Focus::Composer;
                }
                _ => {}
            }
            self.mark();
            return;
        }
        if !matches!(self.modal, Modal::Question | Modal::Tasks | Modal::Target | Modal::History | Modal::Queue) {
            match key.code {
                KeyCode::Home | KeyCode::End => {
                    self.modal_scroll=if key.code==KeyCode::Home{0}else{self.modal_scroll_limit};self.mark();return;
                }
                KeyCode::Up => {
                    self.modal_scroll = self.modal_scroll.saturating_sub(1);
                    self.mark();
                    return;
                }
                KeyCode::Down => {
                    self.modal_scroll = self.modal_scroll.saturating_add(1).min(self.modal_scroll_limit);
                    self.mark();
                    return;
                }
                _ => {}
            }
        }
        if self.live.is_some() {
            self.handle_live_modal(key);
            return;
        }
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right) {
            let max = match self.modal {
                Modal::Scope => 3,
                Modal::Question if self.journal.first != First::Question => 1,
                Modal::Question | Modal::Detach | Modal::Pause | Modal::Cancel | Modal::UnknownCommand => 2,
                Modal::Target => 3,
                Modal::Review => if self.candidate_read() { 4 } else { 2 },
                Modal::Settings => 3,
                _ => 1,
            };
            self.modal_action = if matches!(key.code, KeyCode::Tab | KeyCode::Right) {
                (self.modal_action + 1) % max
            } else {
                (self.modal_action + max - 1) % max
            };
            self.mark();
            return;
        }
        match self.modal {
            Modal::Question => {
                if matches!(key.code, KeyCode::Up | KeyCode::Down) {
                    let old = self
                        .question_choice
                        .unwrap_or(if key.code == KeyCode::Down { 2 } else { 0 });
                    self.question_choice = Some(if key.code == KeyCode::Down {
                        (old + 1) % 3
                    } else {
                        (old + 2) % 3
                    });
                    self.mark();
                    return;
                }
                if let KeyCode::Char(c @ '1'..='3') = key.code {
                    self.question_choice = Some((c as u8 - b'1') as usize);
                    self.mark();
                    return;
                }
                if self.question_choice == Some(2) {
                    match key.code {
                        KeyCode::Backspace => {
                            self.question_text.pop();
                            self.mark();
                            return;
                        }
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if self.question_text.len() < 2048 {
                                self.question_text.push(c);
                                self.mark();
                            }
                            return;
                        }
                        _ => {}
                    }
                }
            }
            Modal::Review if self.modal_action == 3 && self.candidate_read() => match key.code {
                KeyCode::Backspace => {
                    self.reject_reason.pop();
                    self.mark();
                    return;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if self.reject_reason.len() < 1024 {
                        self.reject_reason.push(c);
                        self.mark();
                    }
                    return;
                }
                _ => {}
            },
            Modal::Settings => match key.code {
                KeyCode::Char('t') => {
                    self.journal.theme = match self.journal.theme.as_str() {
                        "dark" => "light",
                        "light" => "mono",
                        _ => "dark",
                    }
                    .into();
                    self.mark();
                    return;
                }
                KeyCode::Char('e') => {
                    self.journal.effects = match self.journal.effects.as_str() {
                        "subtle" => "reduced",
                        "reduced" => "off",
                        _ => "subtle",
                    }
                    .into();
                    self.effect = None;
                    self.mark();
                    return;
                }
                _ => {}
            },
            Modal::Tasks => {
                if key.code == KeyCode::Down && self.journal.second != Second::None {
                    self.journal.selected_task = 1;
                    self.mark();
                    return;
                }
                if key.code == KeyCode::Up {
                    self.journal.selected_task = 0;
                    self.mark();
                    return;
                }
            }
            Modal::Target => {
                if key.code == KeyCode::Down {
                    self.modal_action = (self.modal_action + 1) % 3;
                    self.mark();
                    return;
                }
                if key.code == KeyCode::Up {
                    self.modal_action = (self.modal_action + 2) % 3;
                    self.mark();
                    return;
                }
            }
            _ => {}
        }
        if key.code != KeyCode::Enter {
            return;
        }
        match self.modal {
            Modal::Scope if self.modal_action == 1 => {
                if self.journal.authorize(self.journal.selected_task) {
                    self.modal = Modal::None;
                    self.schedule();
                    self.notice = "Scope authorized in scripted journal".into();
                } else {
                    self.notice = "Scope changed; reopen and review before authorizing".into();
                }
            }
            Modal::Scope if self.modal_action == 2 => self.open(Modal::Cancel),
            Modal::Question if self.modal_action == 1 => {
                if self.journal.first != First::Question {
                    self.modal_action = 0;
                    self.notice = "No current question to answer".into();
                    self.mark();
                    return;
                }
                let value = match self.question_choice {
                    Some(0) => "Allow none".to_owned(),
                    Some(1) => "Require one".to_owned(),
                    Some(2) => self.question_text.trim().to_owned(),
                    _ => String::new(),
                };
                if self.journal.answer(value) {
                    self.modal = Modal::None;
                    self.schedule();
                    self.notice = "Answer saved; reply lost. Reconciling same ID…".into();
                } else {
                    self.notice = "Select a choice or enter Other text before saving".into();
                }
            }
            Modal::Review => match self.modal_action {
                1 => {
                    self.journal.review();
                    self.modal = Modal::Artifact;
                }
                2 if self.candidate_read() => {
                    if self.journal.approve() {
                        self.modal = Modal::None;
                        self.schedule();
                        self.start_effect(Color::Green, 150);
                        self.notice = "Demo approval saved for exact candidate".into();
                    } else {
                        self.notice =
                            "Open the exact candidate first; stale or missing review refused"
                                .into();
                    }
                }
                3 if self.candidate_read() => {
                    if self.journal.reject(&self.reject_reason) {
                        self.modal = Modal::None;
                        self.schedule();
                    } else {
                        self.notice = "Enter a rejection reason".into();
                    }
                }
                2 | 3 => {
                    self.modal_action = 1;
                    self.notice = "Read this exact candidate before deciding".into();
                }
                _ => self.modal = Modal::None,
            },
            Modal::Target => {
                let target = match self.modal_action {
                    0 => Target::Coordinator,
                    1 => Target::NewTask,
                    _ => {
                        if self.journal.selected_task == 1 {
                            Target::SteerSecond
                        } else {
                            Target::SteerFirst
                        }
                    }
                };
                self.set_target(target);
                self.modal = Modal::None;
            }
            Modal::Detach if self.modal_action == 1 => {
                self.detached = true;
                self.modal = Modal::None;
            }
            Modal::Cancel if self.modal_action == 1 => {
                if self.journal.cancel(self.journal.selected_task) {
                    self.modal = Modal::None;
                    self.schedule();
                } else {
                    self.notice =
                        "This selected task cannot be cancelled at its current demo state".into();
                }
            }
            Modal::UnknownCommand if self.modal_action == 1 => {
                self.composer.literal_paste = true;
                self.modal = Modal::None;
                self.submit_composer();
            }
            Modal::Tasks => self.modal = Modal::None,
            Modal::Artifact => self.modal = Modal::Review,
            _ => self.modal = Modal::None,
        }
        self.mark();
    }
    fn handle_live_modal(&mut self, key: KeyEvent) {
        if self.provider_settings(){self.provider_key(key);return;}
        if self.continuous_mode()&&matches!(self.modal,Modal::Queue|Modal::QuestionConfirm){
          let rows=self.queue_rows();self.queue_index=self.queue_index.min(rows.len().saturating_sub(1));
          if self.modal==Modal::Queue&&key.code==KeyCode::Char('e'){
            if self.restored_queue_input.is_some(){self.restore_pending_queue_input();}else if let Some(row)=rows.get(self.queue_index){self.queue_withdraw(row,true);}return;
          }
          if self.modal==Modal::Queue&&matches!(key.code,KeyCode::Up|KeyCode::Down){self.queue_index=if key.code==KeyCode::Down{(self.queue_index+1).min(rows.len().saturating_sub(1))}else{self.queue_index.saturating_sub(1)};self.modal_action=0;self.modal_scroll=0;self.mark();return}
          let max=if self.modal==Modal::Queue{if rows.get(self.queue_index).is_some_and(|r|r["status"]=="held"&&!r["input_id"].is_string())&&self.can_release_held(){3}else if rows.get(self.queue_index).is_some_and(Self::queue_editable){2}else{1}}else{self.confirmation_controls().len()};
          if matches!(key.code,KeyCode::Tab|KeyCode::BackTab|KeyCode::Left|KeyCode::Right){self.modal_action=if matches!(key.code,KeyCode::Tab|KeyCode::Right){(self.modal_action+1)%max}else{(self.modal_action+max-1)%max};self.mark();return}
          if key.code==KeyCode::Enter{
            if self.modal_action==0{self.modal=Modal::None;}
            else if self.modal==Modal::Queue {if let Some(row)=rows.get(self.queue_index){if self.modal_action==1{self.queue_withdraw(row,false);}else{self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"continue_held","payload":{"turn_ids":[row["turn_id"]],"acknowledged_outcome":self.dialogue_state()["pause"]}})));self.notice="Queue action pending durable acceptance".into();}}}
            else if self.confirmation_current()&&self.modal_action+1==self.confirmation_controls().len() {if let Some(a)=self.question_confirmation.clone().filter(|_|self.confirmation_current()){let sha=waywr_backend::codec::canonical_sha256(&a["body"]).unwrap_or_default();self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"confirm_question_disposition","payload":{"action_id":a["action_id"],"suggestion_sha256":sha,"answer":self.scope_editor.text(),"coordination_version":if self.coordination_mode(){serde_json::json!(1)}else{Value::Null}}})));self.notice="Formal decision pending durable acceptance; no call".into();self.modal=Modal::None;}}
            self.mark();return
          }
          return
        }

        // Holding Enter must not advance through acknowledgements or authorize.
        if self.modal==Modal::Scope&&self.scope_mode()&&key.code==KeyCode::Enter&&key.kind!=KeyEventKind::Press{return;}
        if matches!(key.code,KeyCode::Tab|KeyCode::BackTab) || (self.modal!=Modal::Worker&&matches!(key.code,KeyCode::Left|KeyCode::Right)) {
            let max=match self.modal { Modal::Settings if self.native_dialogue_blocked()=>1, Modal::Scope if self.scope_mode()=>self.scope_actions().len(), Modal::Question if self.saved_scope_answer().is_some()=>2, Modal::Question if self.pending_choice().is_none() && self.pending_conversation_question().is_none()=>1,
                Modal::Target=>if self.scope_mode(){2}else{3},
                Modal::Scope|Modal::Question|Modal::Consultation|Modal::Steering|Modal::Settings|Modal::Cancel|Modal::Detach|Modal::Pause|Modal::UnknownCommand=>2,
                Modal::Review=>if self.candidate_read(){4}else{2}, _=>1 };
            self.modal_action=if matches!(key.code,KeyCode::Tab|KeyCode::Right) {(self.modal_action+1)%max}
                else {(self.modal_action+max-1)%max};
            self.mark(); return;
        }
        match self.modal {
            Modal::Question => {
                let conversation=self.pending_conversation_question().is_some();
                let count=if conversation{self.pending_conversation_question()
                    .and_then(|v|v["options"].as_array()).map_or(0,Vec::len)+1}
                    else{self.pending_choice().and_then(|v|v["question"]["options"].as_array()).map_or(0,Vec::len)};
                if matches!(key.code,KeyCode::Up|KeyCode::Down) {
                    if count==0 { return; }
                    let old=self.question_choice.unwrap_or(count-1);
                    self.question_choice=Some(if key.code==KeyCode::Down {(old+1)%count}
                        else {(old+count-1)%count});
                    self.mark(); return;
                }
                if let KeyCode::Char(c @ '1'..='9')=key.code {
                    let selected=(c as u8-b'1') as usize;
                    if selected<count { self.question_choice=Some(selected); self.mark(); return; }
                }
                if conversation && self.question_choice==Some(count-1){
                    match key.code{
                        KeyCode::Backspace=>{self.question_text.pop();self.mark();return;},
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL)=>{
                            if self.question_text.len()<2048{self.question_text.push(c);self.mark();}return;
                        },_=>{}
                    }
                }
            },
            Modal::Review if self.modal_action==3 && self.candidate_read() => match key.code {
                KeyCode::Backspace=>{self.reject_reason.pop();self.mark();return;},
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL)=>{
                    if self.reject_reason.len()<1024 {self.reject_reason.push(c);self.mark();} return;
                },_=>{}
            },
            Modal::Tasks => {
                let len=self.live.as_ref().map_or(0,|v|v.tasks.len());
                if key.code==KeyCode::Down {self.select_task((self.journal.selected_task+1).min(len.saturating_sub(1)));return;}
                if key.code==KeyCode::Up {self.select_task(self.journal.selected_task.saturating_sub(1));return;}
            },
            Modal::Worker => {
                let len=self.live.as_ref().map_or(0,|v|v.operations.len());
                if len>0 && matches!(key.code,KeyCode::Left|KeyCode::Right){
                    if let Some(live)=self.live.as_mut(){
                        live.worker_index=if key.code==KeyCode::Right{(live.worker_index+1)%len}
                            else{(live.worker_index+len-1)%len};
                        live.worker_output.clear();live.worker_cursor=Value::Null;
                        live.worker_terminal=false;live.worker_limit=None;
                        live.worker_at_end=false;
                    }
                    self.modal_scroll=0;self.intent(LiveIntent::Output);self.mark();return;
                }
            },
            Modal::Artifact if key.code==KeyCode::Char('n') => {
                if self.live.as_ref().is_some_and(|live|!live.artifact_complete){
                    self.intent(LiveIntent::OpenArtifactNext);
                }
                return;
            },
            Modal::Settings => {
                if key.code==KeyCode::Char('t') {
                    self.journal.theme=match self.journal.theme.as_str(){"dark"=>"light","light"=>"mono",_=>"dark"}.into();
                    self.mark();return;
                }
                if key.code==KeyCode::Char('e') {
                    self.journal.effects=match self.journal.effects.as_str(){"subtle"=>"reduced","reduced"=>"off",_=>"subtle"}.into();
                    self.effect=None;self.mark();return;
                }
            },
            Modal::Target if self.scope_mode()=>{
                if matches!(key.code,KeyCode::Up|KeyCode::Down){self.modal_action=(self.modal_action+1)%2;self.mark();return;}
            },
            Modal::Target => {
                if key.code==KeyCode::Down {self.modal_action=(self.modal_action+1)%3;self.mark();return;}
                if key.code==KeyCode::Up {self.modal_action=(self.modal_action+2)%3;self.mark();return;}
            }, _=>{}
        }
        if key.code!=KeyCode::Enter {return;}
        match self.modal {
            Modal::Target if self.scope_mode()=>{self.set_target(if self.modal_action==0{Target::Coordinator}else{Target::NewTask});self.modal=Modal::None;},
            Modal::Settings if self.modal_action==1 && self.live.as_ref().is_some_and(|v|v.conversation.is_some())=>{
                if self.chat().is_some_and(|v|v["authorized"]==true){self.notice="Existing allowance remains saved; selecting it again does not add calls".into();return;}
                self.intent(LiveIntent::ReplaceCoordinator);self.modal=Modal::None;
                self.notice=if self.chat().is_some(){"Coordinator authorization pending durable receipt…"}else{"Mock coordinator replacement pending durable receipt…"}.into();
            },
            Modal::Scope if self.scope_mode()=>self.scope_action(),
            Modal::Question if self.saved_scope_answer().is_some()&&self.pending_conversation_question().is_none()&&self.modal_action==1=>{if self.chat().is_some_and(|v|matches!(v["status"].as_str(),Some("available"|"running"))){self.intent(LiveIntent::ContinueAnswer);self.modal=Modal::None;}else{self.notice="Answer saved; coordinator allowance is unavailable or exhausted".into();}},
            Modal::Scope if self.modal_action==1 => {
                self.intent(LiveIntent::Authorize);self.modal=Modal::None;
                self.notice="Authorization pending durable receipt…".into();
            },
            Modal::Consultation if self.modal_action==1=>{
                if self.pending_consultation().is_some(){
                    self.intent(LiveIntent::Consult);self.modal=Modal::None;
                    self.notice="Consultation authorization pending durable receipt…".into();
                }else{self.notice="No consultation request is pending".into();}
            },
            Modal::Steering if self.modal_action==1=>{
                if self.pending_steering().is_some(){
                    self.intent(LiveIntent::ApplySteering);self.modal=Modal::None;
                    self.notice="Steering application pending durable receipt…".into();
                }else{self.notice="No steering suggestion is pending for this task".into();}
            },
            Modal::Question if self.modal_action==1 => {
                if self.coordination_mode()&&self.question_modal_key!=self.pending_conversation_question().map(|q|serde_json::json!({"question_id":q["question_id"],"revision":q["scope"]["question_revision"]})){self.notice="Question changed; your answer is retained. Reopen the current question before saving.".into();return;}
                let question=self.pending_conversation_question().or_else(||self.pending_choice());
                let Some(question) = question else {
                    self.modal_action = 0;
                    self.notice = "No current question to answer; inspect the selected task".into();
                    self.mark();
                    return;
                };
                let conversation=self.pending_conversation_question().is_some();
                let options=if conversation{question["options"].as_array()}else{question["question"]["options"].as_array()};
                let count=options.map_or(0,Vec::len);
                let answer=match self.question_choice {
                    Some(index) if index<count=>options.and_then(|v|v[index].as_str()).unwrap_or("").to_owned(),
                    Some(index) if conversation && index==count=>self.question_text.trim().to_owned(),
                    _=>String::new(),
                };
                if answer.is_empty() { self.notice="Select a choice or enter an answer".into(); }
                else { if self.coordination_mode(){let p=serde_json::json!({"question_id":question["question_id"],"question_revision":question["scope"]["question_revision"],"answer":answer,"coordination_version":1});self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"answer_conversation","payload":p})));}else{self.intent(LiveIntent::Answer(answer));}self.modal=Modal::None;self.notice="Answer pending durable receipt…".into(); }
            },
            Modal::Review => match self.modal_action {
                1=>self.intent(LiveIntent::OpenArtifact),
                2 if self.candidate_read()=>{self.intent(LiveIntent::Approve);self.modal=Modal::None;},
                3 if self.candidate_read()=>{if self.reject_reason.trim().is_empty(){self.notice="Enter a rejection reason".into();}
                    else {self.intent(LiveIntent::Reject(self.reject_reason.clone()));self.modal=Modal::None;}},
                2|3=>{self.modal_action=1;self.notice="Read this exact candidate before deciding".into();},
                _=>self.modal=Modal::None,
            },
            Modal::Detach if self.modal_action==1 => {self.intent(LiveIntent::Detach);self.detached=true;self.modal=Modal::None;},
            Modal::Pause if self.modal_action==1 => {self.intent(LiveIntent::Pause);self.modal=Modal::None;},
            Modal::Cancel if self.modal_action==1 => {self.intent(LiveIntent::Cancel);self.modal=Modal::None;},
            Modal::Target => {
                let target=match self.modal_action {0=>Target::Coordinator,1=>Target::NewTask,_=>Target::SteerFirst};
                self.set_target(target);self.modal=Modal::None;
            },
            Modal::UnknownCommand if self.modal_action==1 => {
                self.composer.literal_paste=true;self.modal=Modal::None;self.submit_composer();
            },
            Modal::Artifact=>{
                self.modal=Modal::Review;
                self.modal_action=if self.candidate_read(){2}else{1};
            },
            _=>self.modal=Modal::None,
        }
        self.mark();
    }
    pub fn input(&mut self, event: Event) {
        // Leaving outranks every editor/confirmation/pending-operation state.
        if let Event::Key(k)=&event&&k.kind!=KeyEventKind::Release&&k.modifiers.contains(KeyModifiers::CONTROL)&&
            (k.code==KeyCode::Char('q') || (k.code==KeyCode::Char('d')&&self.modal==Modal::None&&self.composer.text().is_empty())){
            self.detach_now();return;
        }
        if self.coordination_mode()&&self.modal==Modal::None&&self.focus==Focus::Composer&&(self.draft_reply.is_none()||self.composer.text().is_empty()){
            let kind=match &event{Event::Paste(_)=>Some("paste"),Event::Key(k) if k.kind!=KeyEventKind::Release&&matches!(k.code,KeyCode::Char(_))&&!k.modifiers.intersects(KeyModifiers::CONTROL|KeyModifiers::ALT)=>Some("typed"),_=>None};
            if let Some(kind)=kind{self.draft_reply=Some(self.coordination_context(kind));}
        }
        if self.continuous_mode()&&self.modal==Modal::None&&matches!(&event,Event::Key(k) if k.code==KeyCode::F(7)&&k.kind==KeyEventKind::Press){self.open(Modal::Queue);return}
        if self.modal==Modal::QuestionConfirm&&self.modal_action==1&&self.confirmation_controls().len()==3&&!matches!(&event,Event::Key(k) if matches!(k.code,KeyCode::Tab|KeyCode::BackTab|KeyCode::Esc)) {self.scope_editor.input(event);self.mark();return}
        if matches!(event, Event::Key(_) | Event::Paste(_)) {
            self.cursor_visible = true;
            self.cursor_changed = Instant::now();
            self.dirty = true;
        }
        if let Event::Resize(_, _) = event {
            self.mark();
            return;
        }
        if self.provider_settings()&&self.provider_picker.is_some(){
            if let Event::Paste(value)=&event{let picker=self.provider_picker.as_mut().unwrap();picker.query.input(Event::Paste(value.clone()));picker.index=0;picker.offset=0;self.mark();return;}
        }
        if self.provider_settings()&&self.provider_typing{
            if let Event::Paste(value)=&event{self.scope_editor.input(Event::Paste(value.clone()));self.mark();return;}
        }
        if self.modal==Modal::ScopeEditor{self.scope_editor_input(event);return;}
        if self.modal != Modal::None {
            match event {
                Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_modal(key),
                Event::Paste(value)
                    if self.modal == Modal::Question && (self.live.is_none()&&self.question_choice==Some(2)||self.pending_conversation_question().is_some_and(|q|self.question_choice==Some(q["options"].as_array().map_or(0,Vec::len)))) =>
                {
                    if self.question_text.len() + value.len() <= 2048 {
                        self.question_text.push_str(&value);
                        self.mark();
                    } else {
                        self.notice = "Question text limit is 2 KiB; paste refused".into();
                        self.mark();
                    }
                }
                Event::Paste(value) if self.modal == Modal::Review && self.modal_action == 3 => {
                    if self.reject_reason.len() + value.len() <= 1024 {
                        self.reject_reason.push_str(&value);
                        self.mark();
                    } else {
                        self.notice = "Rejection reason limit is 1 KiB; paste refused".into();
                        self.mark();
                    }
                }
                _ => {}
            }
            return;
        }
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                return;
            }
            match key.code {
                KeyCode::F(1) => {
                    self.open(Modal::Help);
                    return;
                }
                KeyCode::F(3) => {
                    self.open(Modal::Settings);
                    return;
                }
                KeyCode::F(4) => {
                    self.open(Modal::Target);
                    return;
                }
                KeyCode::F(6) => {
                    if self.screen_width >= 110 {
                        self.journal.drawer = !self.journal.drawer;
                        self.mark();
                    } else {
                        self.open(Modal::Tasks);
                    }
                    return;
                }
                KeyCode::F(7) => {
                    self.open(Modal::Worker);
                    return;
                }
                KeyCode::F(8) => {
                    if self.live.is_none() { self.advance(); }
                    return;
                }
                KeyCode::Esc => {
                    if !self.menu&&self.focus==Focus::Composer&&self.composer.text().is_empty(){self.interrupt_coordinator();return;}
                    if !self.menu && self.focus==Focus::Composer {
                        self.composer.clear();self.draft_reply=None;self.history_reply=None;
                        self.input_history.reset_navigation();
                        self.notice="Input cleared".into();
                    }
                    self.menu = false;
                    self.focus = Focus::Composer;
                    self.mark();
                    return;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL)&&self.focus==Focus::Composer=>{
                    if self.composer.text().is_empty(){self.interrupt_coordinator();return;}
                    self.composer.clear();self.draft_reply=None;self.history_reply=None;self.menu=false;
                    self.input_history.reset_navigation();
                    self.notice="Input cleared".into();self.mark();return;
                }
                KeyCode::Tab | KeyCode::BackTab if self.menu => {
                    let list = self.matches();
                    if let Some(command) = list.get(self.menu_index) {
                        self.composer = Composer::from_draft(Draft {
                            text: format!("/{}", command.name),
                            ..Default::default()
                        });
                        if self.live.is_some() { self.composer.area.set_placeholder_text(if self.chat().is_some(){"Message the coordinator…"}else{"Describe a new task request…"}); }
                        self.menu = false;
                        self.mark();
                    }
                    return;
                }
                KeyCode::Up | KeyCode::Down if self.menu => {
                    let count = self.matches().len();
                    if count > 0 {
                        self.menu_index = if key.code == KeyCode::Down {
                            (self.menu_index + 1) % count
                        } else {
                            (self.menu_index + count - 1) % count
                        };
                        self.mark();
                    }
                    return;
                }
                KeyCode::Enter if self.menu && !key.modifiers.contains(KeyModifiers::SHIFT) => {
                    let typed=self.composer.text();
                    if typed.starts_with("/open ") && !typed.contains('\n') && !self.composer.literal_paste{
                        self.menu=false;self.submit_composer();return;
                    }
                    let list = self.matches();
                    if let Some(command) = list.get(self.menu_index) {
                        self.command(command.name);
                    } else {
                        self.open(Modal::UnknownCommand);
                    }
                    return;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    let order = [
                        Focus::Composer,
                        Focus::Send,
                        Focus::Multiline,
                        Focus::Target,
                        Focus::Activity,
                        Focus::Conversation,
                        Focus::Drawer,
                        Focus::Header,
                    ];
                    let here = order.iter().position(|f| *f == self.focus).unwrap_or(0);
                    let offset = if key.code == KeyCode::Tab {
                        1
                    } else {
                        order.len() - 1
                    };
                    self.focus = order[(here + offset) % order.len()];
                    self.mark();
                    return;
                }
                KeyCode::PageUp => {
                    self.scroll_by(true, if key.modifiers.contains(KeyModifiers::ALT){1}else{8});
                    return;
                }
                KeyCode::PageDown => {
                    self.scroll_by(false, if key.modifiers.contains(KeyModifiers::ALT){1}else{8});
                    return;
                }
                KeyCode::Up | KeyCode::Down if self.focus == Focus::Composer && key.modifiers == KeyModifiers::NONE =>
                {
                    let row=self.composer.area.cursor().0;
                    if (key.code==KeyCode::Up&&row>0)||(key.code==KeyCode::Down&&row+1<self.composer.area.lines().len()){
                        self.composer.input(Event::Key(key));self.mark();return;
                    }
                    if !self.input_history.is_recalling(){self.history_reply=Some(self.draft_reply.clone().unwrap_or(Value::Null));}
                    if let Some(draft) = self.input_history.recall(key.code == KeyCode::Up, self.composer.snapshot()) {
                        self.restore_recalled_input(draft);
                    }
                    return;
                }
                KeyCode::Up | KeyCode::Down if self.focus == Focus::Composer && key.modifiers == KeyModifiers::ALT =>
                {
                    self.composer.input(Event::Key(KeyEvent { modifiers: KeyModifiers::NONE, ..key }));
                    self.mark();
                    return;
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.open(Modal::History);
                    return;
                }
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.mark();
                    return;
                }
                KeyCode::Enter if key.kind==KeyEventKind::Press&&key.modifiers.contains(KeyModifiers::ALT)&&self.focus==Focus::Composer=>{
                    self.submit_input(true);return;
                }
                _ => {}
            }
            if self.focus != Focus::Composer {
                match key.code {
                    KeyCode::Left|KeyCode::Right if self.focus==Focus::Activity&&self.coordination_mode()&&(self.coordination_card().is_some()||self.question_suggestion().is_some())=>{let n=if self.coordination_card().is_some(){5}else{2};self.card_action=if key.code==KeyCode::Right{(self.card_action+1)%n}else{(self.card_action+n-1)%n};self.mark();},
                    KeyCode::Enter if key.kind==KeyEventKind::Press => match self.focus {
                        Focus::Send => self.submit_composer(),
                        Focus::Multiline => {
                            self.composer.multiline = !self.composer.multiline;
                            self.mark();
                        }
                        Focus::Target => self.open(Modal::Target),
                        Focus::Activity => self.primary(),
                        Focus::Conversation => self.open(Modal::Inspect),
                        Focus::Drawer => self.open(Modal::Tasks),
                        Focus::Header => self.open(Modal::Settings),
                        _ => {}
                    },
                    KeyCode::Up if self.focus == Focus::Drawer => {
                        self.select_task(self.journal.selected_task.saturating_sub(1));
                    }
                    KeyCode::Down
                        if self.focus == Focus::Drawer && (self.live.as_ref().is_some_and(|v|v.tasks.len()>1) || self.journal.second != Second::None) =>
                    {
                        let max=self.live.as_ref().map_or(1,|v|v.tasks.len().saturating_sub(1));
                        self.select_task((self.journal.selected_task+1).min(max));
                    }
                    _ => {}
                }
                return;
            }
        }
        if self.focus != Focus::Composer && matches!(event, Event::Paste(_)) {
            return;
        }
        if self.focus==Focus::Composer && self.live.is_some() && !self.menu &&
            self.composer.text().trim().is_empty() && !self.composer.multiline &&
            matches!(&event,Event::Key(key) if key.code==KeyCode::Enter &&
                !key.modifiers.contains(KeyModifiers::SHIFT) && key.kind==KeyEventKind::Press){
            self.primary();return;
        }
        let before = self.composer.snapshot();
        match self.composer.input(event) {
            EditResult::Changed => {
                let text = self.composer.text();
                if text.is_empty(){self.draft_reply=None;}
                self.menu =
                    text.starts_with('/') && !text.contains('\n') && !self.composer.literal_paste;
                self.menu_index = 0;
                self.mark();
            }
            EditResult::Submit(text) => {
                // Restore until the command has been classified; a disabled target must keep the draft.
                self.composer = Composer::from_draft(Draft { text, ..before });
                if self.live.is_some() { self.composer.area.set_placeholder_text(if self.chat().is_some(){"Message the coordinator…"}else{"Describe a new task request…"}); }
                self.submit_composer();
            }
            EditResult::Refused(reason) => {
                self.notice = reason.into();
                self.mark();
            }
            EditResult::Nothing => {}
        }
    }

    fn colors(&self) -> (Color, Color, Color, Color) {
        match self.journal.theme.as_str() {
            "light" => (Color::Black, Color::White, Color::Blue, Color::Yellow),
            "mono" => (Color::Reset, Color::Reset, Color::Reset, Color::Reset),
            _ => (Color::White, Color::Reset, Color::Cyan, Color::Yellow),
        }
    }
    fn status_color(&self) -> Color {
        let tone = Tone::for_theme(&self.journal.theme);
        if self.live.is_some() {
            return match self.selected_status().as_str() {
                "approved"=>tone.success,
                "rejected"|"failed"|"cancelled"=>tone.failure,
                "uncertain"|"blocked"|"awaiting-input"=>tone.attention,
                _=>tone.accent,
            };
        }
        if self.journal.selected_task == 1 {
            match self.journal.second {
                Second::Uncertain => tone.attention,
                Second::Cancelled => tone.failure,
                Second::Recovered => tone.success,
                Second::Recovering => tone.attention,
                _ => tone.accent,
            }
        } else {
            match self.journal.first {
                First::Question | First::Review | First::AnswerUnknown => tone.attention,
                First::Approved => tone.success,
                First::Rejected | First::Cancelled => tone.failure,
                _ => tone.accent,
            }
        }
    }
    fn safe(value: &str) -> String {
        value
            .chars()
            .map(|c| match c {
                '\x1b' => "[ESC]".to_owned(),
                '\r' => "[CR]".to_owned(),
                c if c.is_control() && c != '\n' && c != '\t' => format!("[U+{:04X}]", c as u32),
                c => c.to_string(),
            })
            .collect()
    }
    fn visible_message(m: &super::scenario::Message) -> bool {
        // Both transitions remain in the append-only demo journal; the latest
        // delivery state lives in the activity summary instead of two bubbles.
        !m.is_delivery_transition()
    }
    fn activity_time(at: Option<f64>, now: f64) -> String {
        let Some(at) = at.filter(|at| at.is_finite() && *at >= 0.) else { return "time unavailable".into(); };
        let local = |value: f64| {
            let value = value as libc::time_t;
            let mut out: libc::tm = unsafe { std::mem::zeroed() };
            unsafe { libc::localtime_r(&value, &mut out) };
            out
        };
        let at = local(at); let today = local(now);
        if at.tm_year == today.tm_year && at.tm_yday == today.tm_yday {
            format!("{:02}:{:02}", at.tm_hour, at.tm_min)
        } else {
            format!("{:02}/{:02}/{:02} {:02}:{:02}", at.tm_mon + 1, at.tm_mday,
                (at.tm_year + 1900) % 100, at.tm_hour, at.tm_min)
        }
    }
    fn active_work(&self) -> Option<(String, Option<f64>)> {
        if self.live.as_ref()?.phase != "running" { return None; }
        if let Some(turn) = self.queue_rows().into_iter().find(|row| row["status"] == "reserved"&&(!self.coordination_mode()||row["subject_id"]==self.conversation_subject)) {
            return Some(("Coordinator working".into(), turn["started_at"].as_f64()));
        }
        let work = self.development_workflow()?;
        let pending = &work["pending"];
        let dev = self.development_view();
        let workers: Vec<_> = dev["units"].as_array().into_iter().flatten()
            .filter(|unit| unit["work_id"] == work["work_id"] && unit["status"] == "running").collect();
        let stage = match pending["kind"].as_str() {
            Some("coordinate") => "Coordinator working",
            Some("check") => "Running checks",
            _ if !workers.is_empty() => "Working",
            _ => return None,
        };
        let started = pending["started_at"].as_f64().into_iter()
            .chain(workers.iter().filter_map(|unit| unit["started_at"].as_f64()))
            .min_by(f64::total_cmp);
        let label = if workers.is_empty() { stage.into() } else { format!("{stage} · {} workers", workers.len()) };
        Some((label, started))
    }
    fn active_workers(&self) -> Vec<Value> {
        if self.live.as_ref().is_none_or(|live|live.phase != "running") { return Vec::new(); }
        let Some(work) = self.development_workflow() else { return Vec::new(); };
        self.development_view()["units"].as_array().into_iter().flatten()
            .filter(|unit|unit["work_id"] == work["work_id"] && unit["status"] == "running")
            .cloned().collect()
    }
    pub fn has_active_workers(&self) -> bool { !self.active_workers().is_empty() }
    fn worker_activity_lines(&self) -> Vec<Line<'static>> {
        let Some(live) = self.live.as_ref() else { return Vec::new(); };
        let selected = live.selected(self.journal.selected_task).and_then(|task|task["task_id"].as_str());
        let fresh = live.activity_task_id.as_deref() == selected && self.journal.connection == "connected" &&
            live.activity_fetched.is_some_and(|at|at.elapsed() < Duration::from_secs(5));
        let tone = Tone::for_theme(&self.journal.theme);
        let mut lines = Vec::new();
        for (index, unit) in self.active_workers().iter().enumerate() {
            let title = ["title", "name", "unit_id"].into_iter().filter_map(|key|unit[key].as_str())
                .find(|value| !value.is_empty()).unwrap_or("Worker");
            let operation = live.worker_activities.iter().find(|op|op["stage"] == unit["operation_id"] &&
                (unit["attempt_id"].is_null() || unit["attempt_id"] == op["attempt_id"]));
            let recovery_label=unit["recovery"].is_object().then(||format!("Recovery {} · {}",unit["recovery"]["phase"].as_str().unwrap_or("observed"),unit["recovery"]["detail"].as_str().unwrap_or("Evidence retained"))).filter(|_|unit["recovery"]["phase"]!="resolved");
            let activity = if let Some(label)=recovery_label.as_deref(){label}else if fresh { operation.and_then(|op|op["activity"]["label"].as_str())
                .unwrap_or("Working · waiting for activity details") } else { "Working · activity details unavailable" };
            let prefix = if index == 0 { "  └ • " } else { "    • " };
            // Plain, bounded metadata; command output remains exclusively in /output.
            let title = Self::safe(title).split_whitespace().collect::<Vec<_>>().join(" ").chars().take(120).collect::<String>();
            lines.push(Line::styled(format!("{prefix}{title}"), Style::default().fg(tone.text)));
            lines.push(Line::styled(format!("      {}", Self::safe(activity)), Style::default().fg(tone.muted)));
            if fresh {if let Some(usage)=operation.map(|op|&op["activity"]["usage"]).filter(|u|u.is_object()){
                let remaining=match (usage["context_window"].as_u64(),usage["context_tokens"].as_u64()){
                    (Some(window),Some(used))=>format!("{} context tokens remaining (estimate)",window.saturating_sub(used)),_=>"context remaining unknown".into()};
                let cost=usage["cost"].as_f64().map(|c|format!("${c:.4} (estimated/reported)")).unwrap_or_else(||"unknown".into());
                lines.push(Line::styled(format!("      tokens in {} · out {} · cost {} · {}",usage["input"],usage["output"],cost,remaining),Style::default().fg(tone.muted)));
            }}
        }
        lines
    }
    fn active_work_line(&self, now: f64) -> Option<Line<'static>> {
        let (label, started) = self.active_work()?;
        let tone = Tone::for_theme(&self.journal.theme);
        let animate = self.journal.effects == "subtle" && self.journal.theme != "mono";
        let phase = (now * 4.) as usize % (label.chars().count() + 4);
        let mut spans = vec![Span::styled("• ", Style::default().fg(tone.accent))];
        for (index, character) in label.chars().enumerate() {
            let style = Style::default().fg(tone.accent);
            spans.push(Span::styled(character.to_string(), if animate && index.abs_diff(phase) > 2 { style.add_modifier(Modifier::DIM) } else { style }));
        }
        let elapsed = started.filter(|at| at.is_finite() && *at <= now).map(|at| {
            let seconds = (now - at).max(0.) as u64;
            if seconds >= 3600 { format!("{}h {}m {}s", seconds / 3600, seconds / 60 % 60, seconds % 60) }
            else if seconds >= 60 { format!("{}m {}s", seconds / 60, seconds % 60) }
            else { format!("{seconds}s") }
        });
        spans.push(Span::styled(format!("{} · /output inspect", elapsed.map(|time| format!(" ({time})")).unwrap_or_default()), Style::default().fg(tone.muted)));
        Some(Line::from(spans))
    }
    pub fn coordinator_activity_request(&self)->Option<Value>{
        let c=self.live.as_ref()?.conversation.as_ref()?;
        c["turns"].as_array()?.iter().find(|t|t["status"]=="reserved"&&
            (!self.coordination_mode()||t["subject_id"]==self.conversation_subject))
            .map(|t|serde_json::json!({"turn_id":t["turn_id"]}))
    }
    pub fn apply_coordinator_activity(&mut self,body:&Value){
        if let Some(turn)=self.live.as_mut().and_then(|l|l.conversation.as_mut())
            .and_then(|c|c["turns"].as_array_mut()).and_then(|a|a.iter_mut().find(|t|
                t["turn_id"]==body["turn_id"]&&t["status"]==body["status"])){
            let previous=turn["runtime"]["observed_at"].as_f64().unwrap_or(0.);
            if body["runtime"].is_null() || body["runtime"]["observed_at"].as_f64().is_some_and(|at|at>=previous){
                turn["runtime"]=body["runtime"].clone();self.dirty=true;
            }
        }
    }
    fn coordinator_activity_lines(&self) -> Vec<Line<'static>> {
        let Some(live)=self.live.as_ref() else{return Vec::new();};
        let Some(turns)=live.conversation.as_ref().and_then(|c|c["turns"].as_array()) else{return Vec::new();};
        let in_subject=|t:&&Value|!self.coordination_mode()||t["subject_id"]==self.conversation_subject;
        // A pending call must not borrow the previous call's totals or model.
        let turn=turns.iter().filter(in_subject).find(|t|t["status"]=="reserved")
            .or_else(||turns.iter().filter(in_subject).find(|t|t["runtime"].is_object()));
        let Some(turn)=turn else{return Vec::new();};
        let runtime=&turn["runtime"];
        let tone=Tone::for_theme(&self.journal.theme);
        let status=turn["status"].as_str().unwrap_or("unknown");
        let observed=if self.journal.connection=="connected"{"Coordinator call"}else{"Last observed coordinator call"};
        if !runtime.is_object(){return vec![Line::styled(format!("{observed} · {status} · usage not reported yet"),Style::default().fg(tone.muted))];}
        let stale=status=="reserved"&&runtime["observed_at"].as_f64().is_none_or(|at|waywr_backend::execution::now()-at>5.);
        let activity=if stale{"Activity refresh unavailable"}else if status=="reserved"{runtime["activity"]["label"].as_str().unwrap_or("Activity unavailable")}else{""};
        let usage=&runtime["usage"];
        let count=|key:&str|usage[key].as_u64().map(|n|n.to_string()).unwrap_or_else(||"unknown".into());
        let cost=usage["cost"].as_f64().filter(|n|n.is_finite()&&*n>=0.).map(|n|format!("${n:.4} (estimate)")).unwrap_or_else(||"unknown".into());
        let context=match (usage["context_window"].as_u64().filter(|n|*n>0),usage["context_tokens"].as_u64()){
            (Some(window),Some(used))=>format!("~{} / {} tokens left (estimate at last response)",window.saturating_sub(used),window),
            _=>"remaining unknown".into(),
        };
        vec![
            Line::styled(Self::safe(&format!("{observed} · {status} · {}/{} · thinking {}{}",
                runtime["provider"].as_str().unwrap_or("unknown"),runtime["model"].as_str().unwrap_or("unknown"),
                runtime["thinking"].as_str().unwrap_or("unknown"),if activity.is_empty(){String::new()}else{format!(" · {activity}")})),Style::default().fg(tone.muted)),
            Line::styled(format!("Tokens in {} · out {} · cache read {} / write {} · cost {}",count("input"),count("output"),count("cacheRead"),count("cacheWrite"),cost),Style::default().fg(tone.muted)),
            Line::styled(format!("Context {context} · account quota unavailable"),Style::default().fg(tone.muted)),
        ]
    }
    fn conversation_text(&self, width: u16) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        if self
            .journal
            .messages
            .iter()
            .all(|m| !Self::visible_message(m))
        {
            if self.live.is_some() {
                if self.live.as_ref().is_some_and(|v|v.conversation.is_some()){
                    lines.push(Line::from(if self.chat().is_some(){"Converse with your coordinator. Review Settings (F3) before the first live call."}else{"Ask the mock coordinator about this project, or request a task proposal."}));
                    lines.push(Line::from("Discussion does not authorize work. Type / for available actions."));
                }else{
                    lines.push(Line::from("Describe a new task. The current simulated coordinator prepares a scope for your review."));
                    lines.push(Line::from("Open-ended coordinator conversation is not connected yet. Type / for available actions."));
                }
            } else {
                lines.push(Line::from("Ask the coordinator about your project, or describe a task."));
                lines.push(Line::from("A proposed task needs your authorization before it runs. Type / for actions."));
            }
        } else {
            for m in self
                .journal
                .messages
                .iter()
                .filter(|m| Self::visible_message(m))
            {
                lines.extend(self.message_text(m, width).lines);
            }
        }
        if let Some(stream)=self.live.as_ref().and_then(|v|v.conversation.as_ref()).and_then(|v|v.get("provisional")).filter(|v|!v.is_null()){
            let text=stream["text"].as_str().unwrap_or("");
            if !text.is_empty(){lines.push(Line::styled("• Coordinator · provisional",Style::default().fg(Tone::for_theme(&self.journal.theme).accent)));
                lines.extend(Self::result_lines(&Self::safe(text),width,Tone::for_theme(&self.journal.theme)));}
        }
        if let Some(line) = self.active_work_line(waywr_backend::execution::now()) {
            lines.push(line);
            lines.extend(self.worker_activity_lines());
            lines.push(Line::from(""));
        }
        lines.extend(self.coordinator_activity_lines());
        Text::from(lines)
    }
    fn user_row(value: &str, width: u16, background: Color, foreground: Color) -> Line<'static> {
        let mut display = String::from(value);
        let used = value.chars().map(|c| UnicodeWidthChar::width(c).unwrap_or(0)).sum::<usize>();
        display.push_str(&" ".repeat((width as usize).saturating_sub(used)));
        Line::styled(display, Style::default().fg(foreground).bg(background))
    }
    fn result_lines(value: &str, width: u16, tone: Tone) -> Vec<Line<'static>> {
        markdown_lines(value, width.saturating_sub(4), tone, false)
            .into_iter()
            .enumerate()
            .map(|(index, mut line)| {
                if let Some(first) = line.spans.first_mut() {
                    if let Some(trimmed) = first.content.strip_prefix("  ") {
                        first.content = trimmed.to_owned().into();
                    }
                }
                let style = line.style;
                let mut spans = vec![Span::raw(if index == 0 { "  └ " } else { "    " })];
                spans.extend(line.spans);
                Line::from(spans).style(style)
            })
            .collect()
    }
    fn message_text(&self, m: &super::scenario::Message, width: u16) -> Text<'static> {
        let mut lines = Vec::new();
        let tone = Tone::for_theme(&self.journal.theme);
        let safe = Self::safe(&m.text);
        if m.speaker.starts_with("You") {
            let background = match self.journal.theme.as_str() {
                "light" => Color::Rgb(220, 225, 233),
                "mono" => Color::Reset,
                _ => Color::Rgb(74, 77, 84),
            };
            let mut first_line = true;
            for raw in safe.split('\n') {
                let mut visual = if first_line { String::from("> ") } else { String::from("  ") };
                let mut used = 2usize;
                for ch in raw.chars() {
                    let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if used + char_width > width as usize && used > 2 {
                        lines.push(Self::user_row(&visual, width, background, tone.text));
                        visual = String::from("  ");
                        used = 2;
                    }
                    visual.push(ch);
                    used += char_width;
                }
                lines.push(Self::user_row(&visual, width, background, tone.text));
                first_line = false;
            }
        } else if self.card_seqs.contains_key(&m.seq) {
            let current=self.coordination_card().is_some_and(|c|c["card_id"].as_str()==self.card_seqs.get(&m.seq).map(String::as_str));
            lines.push(Line::styled(format!("• {} · Proposal · {}",Self::activity_time(m.at,waywr_backend::execution::now()),if current{"Awaiting your approval"}else{"Previous revision"}),Style::default().fg(tone.accent).add_modifier(Modifier::BOLD)));
            lines.extend(Self::result_lines(&safe,width,tone));
        } else if m.speaker.starts_with("Coordinator") {
            lines.push(Line::styled(
                format!("• {} · Coordinator", Self::activity_time(m.at, waywr_backend::execution::now())),
                Style::default()
                    .fg(tone.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.extend(Self::result_lines(&safe, width, tone));
        } else {
            let (label, color) = if let Some(activity)=m.activity{
                use super::scenario::ActivityStatus::*;
                match activity {Activity=>("Activity",tone.accent),Success=>("Passed",tone.success),Failure=>("Failed",tone.failure),Attention=>("Needs attention",tone.attention),Uncertain=>("Uncertain",tone.attention),Cancelled=>("Cancelled",tone.attention)}
            }else if safe.starts_with("Review reviewed ·")||safe.starts_with("Review recorded ·"){
                // Legacy cached prose alone cannot establish a review outcome.
                ("Activity",tone.accent)
            }else if safe.starts_with("Reply unavailable ·"){("Reply unavailable",tone.attention)}else if safe.starts_with("Interrupted ·"){("Interrupted",tone.attention)}else if safe.starts_with("Failed response ·"){("Failed",tone.failure)}else if safe.starts_with("Coordinator interruption requested ·"){("Stopping",tone.attention)}else if safe.contains("uncertain") {
                ("Uncertain", tone.attention)
            } else if safe.contains("blocked") || safe.contains("Coordinator stopped") {
                ("Stopped", tone.attention)
            } else if safe.contains("cancelled") {
                ("Cancelled", tone.attention)
            } else if safe.contains("rejected") || safe.contains("failed") || safe.contains("output-invalid") {
                ("Failed", tone.failure)
            } else if safe.contains("question") || safe.contains("Answer saved") {
                ("Needs attention", tone.attention)
            } else if self.development_mode() {
                ("Activity",tone.accent)
            } else if safe.contains("approved")
                || safe.contains("restored")
                || safe.contains("retained")
            {
                ("Done", tone.success)
            } else {
                ("Working", tone.accent)
            };
            let (heading, result) = safe
                .split_once(" · ")
                .unwrap_or(("Activity", safe.as_str()));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("• {} · {label} · ", Self::activity_time(m.at, waywr_backend::execution::now())),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    heading.to_owned(),
                    Style::default().fg(tone.text).add_modifier(Modifier::BOLD),
                ),
            ]));
            if !result.is_empty() {
                lines.extend(Self::result_lines(result, width, tone));
            }
        }
        lines.push(Line::from(""));
        Text::from(lines)
    }
    fn draw_line(frame: &mut Frame<'_>, area: Rect, text: impl Into<String>, style: Style) {
        frame.render_widget(Paragraph::new(text.into()).style(style), area);
    }
    pub fn render(&mut self, frame: &mut Frame<'_>, elapsed: Duration) {
        let screen = frame.area();
        self.screen_width = screen.width;
        if screen.width < 60 || screen.height < 18 {
            frame.render_widget(Clear, screen);
            frame.render_widget(Paragraph::new(if self.chat().is_some(){"CONVERSATION · Resize to at least 60×18; draft is saved. Ctrl-Q detaches."}else if self.live.is_some() {"SIMULATED SERVICE · Resize to at least 60×18; the project draft remains saved. Ctrl-Q detaches."} else {"SIMULATED · Resize to at least 60×18; the scripted journal and draft remain saved. Ctrl-Q detaches."})
                .wrap(Wrap{trim:false}),screen);
            return;
        }
        let (fg, bg, accent, attention) = self.colors();
        let base = Style::default().fg(fg).bg(bg);
        frame.render_widget(Block::default().style(base), screen);
        let wide = screen.width >= 110;
        let active_model=self.model_header_label();
        let selection_h=active_model.as_ref().map(|s|Paragraph::new(s.as_str()).wrap(Wrap{trim:false}).line_count(screen.width).min(3) as u16).unwrap_or(0);
        let header_h = 1+selection_h;
        let empty_session = self.live.as_ref().map_or(
            self.journal.first == First::Empty && self.journal.second == Second::None,
            |v|v.tasks.is_empty() && v.conversation.as_ref().is_none_or(|c|
                c["messages"].as_array().is_none_or(Vec::is_empty) &&
                c["questions"].as_array().is_none_or(Vec::is_empty) &&
                c["actions"].as_array().is_none_or(Vec::is_empty)));
        let activity_h = if empty_session { 1 } else { 3 };
        let max_editor_h = if screen.height >= 44 {
            6
        } else if screen.height >= 30 {
            5
        } else {
            3
        };
        let usable = screen.width.saturating_sub(7).max(1) as usize;
        let draft_rows: usize = self
            .composer
            .text()
            .split('\n')
            .map(|line| 1 + line.chars().count().saturating_sub(1) / usable)
            .sum();
        let editor_h = draft_rows.clamp(2, max_editor_h) as u16;
        let composer_box_h = editor_h + 2;
        let bottom = activity_h + 1 + 1 + composer_box_h + 1 + 1; // task summary, separator, target, padded composer, toolbar, footer
        let body_y = header_h + 1;
        let body_h = screen.height.saturating_sub(header_h + 1 + bottom);
        let conv_w = if wide && self.journal.drawer {
            screen.width.saturating_sub(45)
        } else {
            screen.width
        };
        let tone = Tone::for_theme(&self.journal.theme);
        let project=self.live.as_ref().map_or("Project Decision Log",|v|v.project.as_str());
        let header = Line::from(vec![
            Span::styled("Wayward Wrangler", base.fg(tone.accent).add_modifier(Modifier::BOLD)),
            Span::styled(if self.development_mode(){" · DEVELOPMENT"}else if self.chat().is_some_and(|v|v["profile"]["mode"]!="mock-cli") {" · NATIVE CONVERSATION"}else if self.chat().is_some(){" · MOCK CLI STREAM"}else if self.live.is_some() {" · SIMULATED SERVICE"} else {" · SIMULATED DEMO"}, base.fg(tone.attention).add_modifier(Modifier::BOLD)),
            Span::styled(self.live.as_ref().filter(|live|live.phase!="running").map(|live|format!(" · Session {}",live.phase)).unwrap_or_default(),base.fg(tone.attention).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" · {project}"), base.fg(fg)),
            Span::styled(if let Some(chat)=self.chat(){if active_model.is_some(){format!(" · {}",chat["status"].as_str().unwrap_or("blocked"))}else{format!(" · {}/{} · {}",chat["profile"]["provider"].as_str().unwrap_or("Claude"),chat["profile"]["model"].as_str().unwrap_or("unconfigured"),chat["status"].as_str().unwrap_or("blocked"))}}else if self.live.is_some(){" · Coordinator: fake/deterministic".into()}else{" · Coordinator: scripted".into()},base.fg(tone.option)),
            Span::styled(if self.provider_state()["switch"]["status"].as_str().is_some_and(|s|matches!(s,"requested"|"waiting"|"blocked")){" · Switch pending"}else{""},base.fg(tone.attention)),
            Span::styled(if self.journal.connection == "connected" { " · Service connected" } else { " · Service reconnecting" }, base.fg(if self.journal.connection == "connected" { tone.success } else { tone.attention })),
            Span::styled(if wide { " · Settings F3" } else { "" }, base.fg(tone.muted)),
        ]);
        frame.render_widget(Paragraph::new(header).style(base), Rect::new(0, 0, screen.width, 1));
        if let Some(label)=active_model{frame.render_widget(Paragraph::new(Self::safe(&label)).wrap(Wrap{trim:false}).style(base.fg(tone.option)),Rect::new(0,1,screen.width,selection_h));}
        Self::draw_line(
            frame,
            Rect::new(0, header_h, screen.width, 1),
            "─".repeat(screen.width as usize),
            Style::default().fg(accent),
        );
        let conv_width = conv_w;
        let text = self.conversation_text(conv_width);
        let p = Paragraph::new(text).wrap(Wrap { trim: false });
        let total = p.line_count(conv_width.max(1));
        let max_top = total.saturating_sub(body_h as usize);
        let mut top = max_top.saturating_sub(self.journal.scroll_back as usize);
        if let Some(anchor) = self.journal.scroll_anchor_seq {
            let mut start = 0usize;
            let mut found = false;
            for message in self
                .journal
                .messages
                .iter()
                .filter(|m| Self::visible_message(m))
            {
                let count = Paragraph::new(self.message_text(message, conv_width))
                    .wrap(Wrap { trim: false })
                    .line_count(conv_width.max(1));
                if message.seq == anchor {
                    top = (start
                        + (self.journal.scroll_anchor_line as usize).min(count.saturating_sub(1)))
                    .min(max_top);
                    found = true;
                    break;
                }
                start += count;
            }
            if !found {
                self.journal.scroll_anchor_seq = None;
                top = max_top;
            }
        }
        self.viewport_top = top;
        self.viewport_total = total;
        self.viewport_height = body_h as usize;
        self.viewport_width = conv_width;
        self.present_visible_card(top,body_h as usize,conv_width);
        self.present_visible_candidate(top,body_h as usize,conv_width);
        let conv = Rect::new(0, body_y, conv_width, body_h);
        frame.render_widget(
            p.scroll((top.min(u16::MAX as usize) as u16, 0)).style(base),
            conv,
        );
        if self.journal.unread > 0 {
            let marker = format!("{} newer events · PageDown latest", self.journal.unread);
            let x = conv_w.saturating_sub(marker.len() as u16 + 1);
            Self::draw_line(
                frame,
                Rect::new(
                    x,
                    body_y + body_h.saturating_sub(1),
                    marker.len().min(conv_w as usize) as u16,
                    1,
                ),
                marker,
                Style::default().fg(attention),
            );
        }
        if wide && self.journal.drawer {
            Self::draw_line(
                frame,
                Rect::new(conv_w, body_y, 1, body_h),
                "│\n".repeat(body_h as usize),
                base,
            );
            let drawer = Rect::new(conv_w + 1, body_y, screen.width - conv_w - 1, body_h);
            frame.render_widget(
                Paragraph::new(self.tasks_text())
                    .wrap(Wrap { trim: false })
                    .style(base),
                drawer,
            );
        }
        let activity_y = body_y + body_h;
        let title = self.coordination_request().and_then(|r|r["scope"]["record"]["proposal"]["title"].as_str().or_else(||r["title"].as_str())).map(str::to_owned).unwrap_or_else(||if self.coordination_mode(){"Current request".into()}else{self.selected_title()});
        let calls = if self.journal.selected_task == 0 {
            self.journal.first_calls
        } else {
            self.journal.second_calls
        };
        let action = self.action_label();
        if empty_session {
            Self::draw_line(
                frame,
                Rect::new(0, activity_y, screen.width, 1),
                if self.live.as_ref().is_some_and(|v|v.conversation.is_some()){
                    if self.development_mode(){"Development · F3 agents/policy · /plan scope approval · /inspect history"}else if self.chat().is_some(){"Conversation only · Settings F3 reviews provider, context and allowance"}else{"No task yet · Discuss with the mock coordinator or request a task proposal"}
                }else if self.live.is_some(){"No task yet · Describe a new task request"}
                else{"No task yet · Ask the coordinator or describe new work"},
                base.fg(accent),
            );
        } else {
            let status_color = self.status_color();
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!("{} · {title}",if self.live.as_ref().is_some_and(|v|v.tasks.is_empty()){"Conversation"}else{"Task"}),
                        base.fg(accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("   [{}]", if self.coordination_mode(){self.coordination_status()}else{self.selected_status()}),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])),
                Rect::new(0, activity_y, screen.width, 1),
            );
            if let Some(live)=&self.live {
                let row=live.selected(self.journal.selected_task);
                if self.continuous_mode()&&!self.development_mode(){let rows=self.queue_rows();Self::draw_line(frame,Rect::new(0,activity_y+1,screen.width,1),format!("  ├─ Running {} · queued {} · held {} · Queue F7 · scope review /plan (optional)",rows.iter().filter(|r|r["status"]=="reserved").count(),rows.iter().filter(|r|r["status"]=="queued").count(),rows.iter().filter(|r|r["status"]=="held").count()),base); }else if let Some(conv)=live.conversation.as_ref().filter(|_|live.tasks.is_empty()&&!self.development_mode()){
                    let budget=&conv["budgets"];
                    Self::draw_line(frame,Rect::new(0,activity_y+1,screen.width,1),
                        format!("  ├─ Conversation calls: {}/{} · consultation: {}/{} · /inspect for details",
                            budget["conversation"]["charged"],budget["conversation"]["limit"],
                            budget["consultation"]["charged"],budget["consultation"]["limit"]),base);
                } else if self.development_mode() {
                    let dev = self.development_view();
                    let workflow = self.development_workflow();
                    let running = dev["units"].as_array().map_or(0, |units| units.iter()
                        .filter(|unit| workflow.as_ref().is_some_and(|work|
                            unit["work_id"] == work["work_id"]) && unit["status"] == "running")
                        .count());
                    let status = workflow.as_ref().map(|work| {
                        if work["pending"]["kind"] == "coordinate" && work["status"] == "running" {
                            "coordinator reviewing"
                        } else if work["pending"]["kind"] == "check" && work["status"] == "running" {
                            "combined checks running"
                        } else { work["status"].as_str().unwrap_or("unknown") }
                    }).unwrap_or(if self.coordination_mode()&&self.current_scope().is_none(){"discussing"}else{"awaiting scope approval"});
                    let recovery=workflow.as_ref().map(|w|&w["recovery"]).filter(|r|r.is_object()&&r["phase"]!="resolved");
                    let detail=if let Some(r)=recovery{format!("{} {} · {} · {} · /inspect evidence",if r["scope"]=="assignment"{"Assignment"}else{"Recovery"},r["phase"].as_str().unwrap_or("observed"),Self::safe(r["detail"].as_str().unwrap_or("Evidence retained")),if r["decision_needed"]==true{"Answer in chat"}else{"No decision needed"})}else{format!("Development: {status} · {running} workers active · {}",if status=="attention"{if self.development_reply_payload("").is_some(){"Answer in chat · /inspect reason"}else{"/inspect concrete blocker"}}else{"F6 tasks · /inspect history"})};
                    Self::draw_line(frame,Rect::new(0,activity_y+1,screen.width,1),format!("  ├─ {detail}"),base);
                }else if self.scope_mode(){Self::draw_line(frame,Rect::new(0,activity_y+1,screen.width,1),format!("  ├─ Scope revision {} · execution unavailable · no workers launched",row.map_or(Value::Null,|v|v["version"].clone())),base);
                }else{
                    let run=row.and_then(|v|v["run_id"].as_str()).unwrap_or("workflow not started");
                    Self::draw_line(frame,Rect::new(0,activity_y+1,screen.width,1),
                        format!("  ├─ Run: {run} · phase: {} · task details via /inspect",live.phase),base);
                }
            } else {
                frame.render_widget(Paragraph::new(Line::from(vec![
                    Span::raw(format!("  ├─ Progress: {calls}/5 fake calls · critiques {}/2 · artifact: ",if self.journal.first_calls >= 2 { 1 } else { 0 })),
                    Span::styled(if self.journal.first == First::Review || self.journal.first == First::Approved {"demo addendum"} else {"revision-2.md"},
                        Style::default().fg(Tone::for_theme(&self.journal.theme).path).add_modifier(Modifier::BOLD)),
                ])),Rect::new(0,activity_y+1,screen.width,1));
            }
            Self::draw_line(
                frame,
                Rect::new(0, activity_y + 2, screen.width, 1),
                format!(
                    "{} [{}] · {}",
                    if self.focus == Focus::Activity {
                        "›"
                    } else {
                        " "
                    },
                    action,
                    if self.live.is_some() {
                        if action == "Review scope" {
                            "Enter with empty input opens scope (no authorization)"
                        } else {
                            "Enter with empty input opens; Tab to focus action"
                        }
                    } else if self.journal.selected_task == 0
                        && self.journal.first == First::Suggested
                    {
                        "Enter opens a proposal; it does not authorize work"
                    } else {
                        "Enter opens this action"
                    }
                ),
                if self.focus == Focus::Activity {
                    base.fg(accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    base.fg(accent)
                },
            );
        }
        if self.coordination_mode()&&self.coordination_card().is_some(){Self::draw_line(frame,Rect::new(0,activity_y+2,screen.width,1),["Details","Approve","Change","Not now","Reject"].iter().enumerate().map(|(i,n)|format!("{} [{n}]",if self.focus==Focus::Activity&&self.card_action==i{"›"}else{" "})).collect::<Vec<_>>().join(" "),base.fg(accent));}
        if self.coordination_mode()&&self.coordination_card().is_none()&&self.question_suggestion().is_some(){Self::draw_line(frame,Rect::new(0,activity_y+2,screen.width,1),["Details","Save quoted answer"].iter().enumerate().map(|(i,n)|format!("{} [{n}]",if self.focus==Focus::Activity&&self.card_action%2==i{"›"}else{" "})).collect::<Vec<_>>().join(" "),base.fg(accent));}
        let separator_y = activity_y + activity_h;
        Self::draw_line(
            frame,
            Rect::new(0, separator_y, screen.width, 1),
            "─".repeat(screen.width as usize),
            Style::default().fg(accent),
        );
        // /new selects a topic. Once that same topic owns a saved scope, the
        // caption follows the bound scope without changing routing or drafts.
        let displayed_target=if self.coordination_mode()&&self.journal.target==Target::NewTask&&self.conversation_scope_task_id().is_some(){Target::Coordinator}else{self.journal.target};
        let target = match displayed_target {
            Target::Coordinator if self.live.is_some() => {
                let provider = self.live.as_ref()
                    .and_then(|live| live.conversation.as_ref())
                    .and_then(|conversation| conversation["binding"]["provider"].as_str());
                let label = if provider == Some("fake") {
                    "Mock coordinator conversation"
                } else {
                    "Coordinator conversation"
                };
                if self.journal.connection == "connected" {
                    if self.continuous_mode(){let state=self.dialogue_state();let bound=state["subjects"].as_array().and_then(|a|a.iter().find(|v|v["subject_id"]==self.conversation_subject)).and_then(|v|v["task_id"].as_str());if let Some(title)=bound.and_then(|id|self.live.as_ref().unwrap().tasks.iter().find(|t|t["task_id"]==id)).and_then(|t|t["title"].as_str()){format!("{label} · scope: {title}")}else{format!("{label} · current topic")}}else if self.scope_mode()&&self.live.as_ref().is_some_and(|v|!v.tasks.is_empty()){format!("{label} · selected scope: {}",self.selected_title())}else{label.to_owned()}
                } else {
                    format!("{label} (reconnecting; draft preserved)")
                }
            }
            Target::Coordinator => "Coordinator conversation".into(),
            Target::NewTask if self.scope_mode()=>"New topic / proposal discussion (no selected scope)".into(),
            Target::NewTask => "New task request".into(),
            Target::SteerFirst if self.live.is_some() && self.journal.connection != "connected" =>
                "Steer selected task (reconnecting; draft preserved)".into(),
            Target::SteerFirst if self.live.is_some() => if self.development_mode(){"In-scope steering · existing approved policy".into()}else{"Steer selected task".into()},
            Target::SteerFirst => "Steer: clarify choices (unavailable)".into(),
            Target::ReplyFirst => "Answer development question · scope/policy unchanged".into(),
            Target::SteerSecond => "Steer: walkthrough (unavailable)".into(),
        };
        let focus_label = if self.focus == Focus::Target {
            ">"
        } else {
            " "
        };
        Self::draw_line(
            frame,
            Rect::new(0, separator_y + 1, screen.width, 1),
            format!("{focus_label} {}To: {target} [Change F4]",if self.continuous_mode()&&!self.composer.text().is_empty(){if self.conversation_revision_request{"Draft revision request · not sent · "}else{"Draft · not sent · "}}else{""}),
            base,
        );
        let composer_bg = match self.journal.theme.as_str() {
            "light" => Color::Rgb(232, 237, 244),
            "mono" => bg,
            _ => Color::Rgb(65, 69, 76),
        };
        let composer_y = separator_y + 2;
        let composer_box = Rect::new(0, composer_y, screen.width, composer_box_h as u16);
        frame.render_widget(
            Block::default().style(Style::default().bg(composer_bg)),
            composer_box,
        );
        Self::draw_line(
            frame,
            Rect::new(0, composer_y + 1, 1, 1),
            "›",
            Style::default()
                .fg(accent)
                .bg(composer_bg)
                .add_modifier(Modifier::BOLD),
        );
        self.composer
            .area
            .set_style(Style::default().fg(fg).bg(composer_bg));
        self.composer.area.set_cursor_line_style(Style::default());
        self.composer.area.set_cursor_style(if self.cursor_visible && self.focus == Focus::Composer {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        });
        self.composer.area.set_placeholder_style(
            Style::default()
                .fg(Tone::for_theme(&self.journal.theme).muted)
                .bg(composer_bg),
        );
        self.composer
            .area
            .set_selection_style(Style::default().add_modifier(Modifier::REVERSED));
        let editor_rect = Rect::new(2, composer_y + 1, screen.width.saturating_sub(3), editor_h);
        frame.render_widget(&self.composer.area, editor_rect);
        let toolbar_y = composer_y + composer_box_h as u16;
        let send=format!("{}[{}]",if self.focus==Focus::Send{">"}else{""},if self.model_setup_needed(){"Use saved model"}else if self.foreground_coordinator().is_some()&&self.live_steering_available(){"Steer"}else{"Send"});
        let multi = if self.composer.multiline { "on" } else { "off" };
        Self::draw_line(
            frame,
            Rect::new(0, toolbar_y, screen.width, 1),
            format!("{send}  [Multiline F2: {multi}]  [Target F4]  [Tasks F6]  [Settings F3]{}",if self.continuous_mode(){"  [Queue F7]"}else{""}),
            base,
        );
        Self::draw_line(
            frame,
            Rect::new(0, toolbar_y + 1, screen.width, 1),
            if !self.notice.is_empty() {
                self.notice.as_str()
            } else if self.focus == Focus::Conversation {
                "PgUp/PgDn chat · Alt+PgUp/PgDn line · Esc composer · Enter inspect"
            } else if self.composer.multiline {
                "↑/↓ cursor/history · Enter newline · Alt+Enter follow-up · Tab Send · Esc/Ctrl-C clear"
            } else if self.composer.text().is_empty() {
                if self.foreground_coordinator().is_some(){"↑/↓ history · Enter action · Esc/Ctrl-C interrupt · Ctrl-Q leave · F1 help"}else{"↑/↓ history · PgUp/PgDn chat · Enter action · Ctrl-Q leave · F1 help"}
            } else if self.foreground_coordinator().is_some()&&self.live_steering_available(){
                "↑/↓ cursor/history · Enter steer · Alt+Enter follow-up · Ctrl-J newline · Esc/Ctrl-C clear · F1 help"
            } else {
                "↑/↓ cursor/history · Enter send · Ctrl-J newline · Esc/Ctrl-C clear · F1 help"
            },
            if self.notice.is_empty() {
                Style::default().fg(fg)
            } else {
                Style::default().fg(attention)
            },
        );
        if self.menu {
            self.render_menu(frame, editor_rect, base, accent);
        }
        if self.modal != Modal::None {
            self.render_modal(frame, screen, base, accent, attention);
        }
        if self.journal.effects == "subtle" {
            if let Some(effect) = self.effect.as_mut() {
                let fx_area = Rect::new(0, 0, screen.width.min(17), 1);
                effect.process(elapsed.into(), frame.buffer_mut(), fx_area);
                if effect.done() {
                    self.effect = None;
                }
            }
        } else {
            self.effect = None;
        }
    }
    fn tasks_text(&self) -> String {
        if let Some(live)=&self.live {
            let mut text="Tasks [F6 hide] · selected project only\n".to_owned();
            let dev=self.development_view();
            for (index,task) in live.tasks.iter().enumerate() {
                let workflow=dev["workflows"].as_array().and_then(|all|all.iter().find(|w|w["work_id"]==task["task_id"]));
                let progress=if self.development_mode(){format!("development {}",workflow.and_then(|w|w["status"].as_str()).unwrap_or("awaiting scope approval"))}else{task["run_id"].as_str().unwrap_or("workflow not started").to_owned()};
                text.push_str(&format!("{} {}\n  {} · {}\n",if index==self.journal.selected_task{">"}else{" "},
                    task["title"].as_str().unwrap_or("Task"),task["status"].as_str().unwrap_or("unknown"),
                    progress));
            }
            if live.tasks.is_empty(){text.push_str("No tasks. Describe a new request.\n");}
            if self.development_mode(){if let Some(workflows)=dev["workflows"].as_array(){for w in workflows{let unresolved=w["obligations"].as_array().map_or(0,|all|all.iter().filter(|o|o["status"]!="resolved").count());text.push_str(&format!("\nDevelopment: {} · {} unresolved obligations\nIntegration: {}\nHead: {}\nAttention: {}\n",w["status"].as_str().unwrap_or("unknown"),unresolved,w["result_branch"].as_str().or(w["integration_branch"].as_str()).unwrap_or("pending"),w["head"],w["attention"]));if let Some(path)=w["integration_checkout"]["directory"].as_str(){text.push_str(&format!("Combined checkout: {}\nUse this directory for preview; main advances only after delivery.\n",Self::safe(path)));}}}if let Some(units)=dev["units"].as_array(){for u in units{text.push_str(&format!("\n  {} · {}\n  {}/{} · {}\n  Dependencies: {}\n  Commit: {} · reported obligations: {}\n",u["title"].as_str().unwrap_or("Task"),u["status"].as_str().unwrap_or("planned"),u["provider"].as_str().unwrap_or("codex"),u["model"].as_str().unwrap_or(""),u["effort"].as_str().unwrap_or(""),u["start_after"],u["result_commit"],u["obligations"]));}}}
            text.push_str("\nF3 → 5 Tasks changes profiles for unreserved attempts; frozen selections remain in Inspect.\nOpen /output for live or retained worker streams. Herdr views are optional and explicit.");
            return text;
        }
        let first = if self.journal.selected_task == 0 {
            ">"
        } else {
            " "
        };
        let second = if self.journal.selected_task == 1 {
            ">"
        } else {
            " "
        };
        let mut s = format!(
            "Tasks [F6 hide]\n{first} Clarify Decision Log choices\n  {} · {} fake calls\n  [Inspect] [Action]\n",
            self.first_short(),
            self.journal.first_calls
        );
        if self.journal.second != Second::None {
            s.push_str(&format!(
                "\n{second} Manual walkthrough\n  {} · {} fake calls\n  [Inspect] [Action]\n",
                self.second_short(),
                self.journal.second_calls
            ));
        }
        s.push_str("\nWorkers · selected task\n");
        if self.journal.first_calls > 0 {
            s.push_str("author · fake · retained [Inspect]\n");
        }
        if self.journal.first_calls > 1 {
            s.push_str("critic · fake · retained [Inspect]\n");
        }
        if self.journal.second_calls > 0 {
            s.push_str("walkthrough · fake · retained\n");
        }
        s
    }
    fn first_short(&self) -> &'static str {
        match self.journal.first {
            First::Empty => "Empty",
            First::PendingDelivery => "Sending",
            First::Coordinator => "Planning",
            First::Suggested => "Suggestion",
            First::Proposed => "Scope ready",
            First::Authorized => "Queued",
            First::Drafting => "Drafting",
            First::Critiquing => "Reviewing",
            First::Question => "Answer needed",
            First::AnswerUnknown => "Reconnecting",
            First::Revising => "Revising",
            First::Review => "Final review",
            First::Approved => "Approved (demo)",
            First::Rejected => "Rejected (demo)",
            First::Cancelled => "Cancelled (demo)",
        }
    }
    fn second_short(&self) -> &'static str {
        match self.journal.second {
            Second::None => "None",
            Second::Proposed => "Scope ready",
            Second::AuthorizedQueued => "Queued",
            Second::Active => "Executing",
            Second::Recovering => "Recovering",
            Second::Recovered => "Recovered",
            Second::Uncertain => "Uncertain",
            Second::Cancelled => "Cancelled",
        }
    }
    fn action_label(&self) -> String {
        if let Some(turn)=self.foreground_coordinator(){return if turn["interrupt_requested"]==true || self.interrupt_pending.as_ref().is_some_and(|(id,_)|turn["turn_id"]==*id){"Stopping coordinator · keep drafting / Ctrl-Q leave".into()}else{"Interrupt coordinator".into()};}
        if self.model_setup_needed(){return "Use saved model".into();}
        if self.coordination_mode(){if self.pending_conversation_question().is_some(){return "Answer in chat".into()}if self.coordination_card().is_some(){return "Proposal details / inline approval".into()}if self.current_scope().is_none(){return "Continue conversation".into()}}
        if self.continuous_mode(){let rows=self.queue_rows();if let Some(r)=rows.iter().find(|r|r["status"]=="reserved"&&(!self.coordination_mode()||r["subject_id"]==self.conversation_subject)){return if r["interrupt_requested"]==true{"Interrupt requested · inspect queue".into()}else{"Interrupt coordinator".into()};}if self.question_suggestion().is_some(){return "Record suggested formal answer".into()}if rows.iter().any(|r|r["status"]=="held"){return "View held messages".into()}}
        if self.chat().is_some_and(|v|v["status"]=="awaiting-authorization"){return "Configure coordinator".into();}
        if let Some(live) = &self.live {
            if self.pending_conversation_question().is_some(){return "Answer question".into();}
            if self.saved_scope_answer().is_some(){return "Continue with coordinator · uses 1 call".into();}
            if self.development_mode()&&self.current_scope().is_some_and(|s|s["authorized"]==true){return "Inspect development and history".into();}
            if self.scope_mode()&&live.status(self.journal.selected_task)=="scope-authorized"{return "View authorized scope".into();}
            if self.pending_consultation().is_some(){return "Review consultation".into();}
            if self.pending_steering().is_some(){return "Review steering".into();}
            if live.tasks.is_empty() && live.conversation.is_some(){
                return "Inspect conversation".into();
            }
            return match live.status(self.journal.selected_task) {
                "proposed" => "Review scope".into(),
                "awaiting-input" => {
                    let selected = live.selected(self.journal.selected_task);
                    let detail = live.detail.as_ref().filter(|detail| {
                        detail["task"]["task_id"]
                            == selected.map_or(Value::Null, |row| row["task_id"].clone())
                            && detail["task"]["version"]
                                == selected.map_or(Value::Null, |row| row["version"].clone())
                            && detail["task"]["state_revision"]
                                == selected.map_or(Value::Null, |row| row["state_revision"].clone())
                    });
                    match detail.and_then(|v| v["pending"]["kind"].as_str()) {
                        Some("choice") => "Answer question",
                        Some("final-review") => "Review candidate",
                        _ => "Loading next action",
                    }
                    .into()
                }
                "approved" | "rejected" | "completed" => "View result".into(),
                "blocked" | "uncertain" | "failed" => "Inspect issue".into(),
                _ => "View progress".into(),
            };
        }
        if self.journal.selected_task == 1 {
            if self.journal.second == Second::Proposed {
                "Review scope".into()
            } else {
                "Inspect task".into()
            }
        } else {
            match self.journal.first {
                First::Suggested => "Create task proposal",
                First::Proposed => "Review scope",
                First::Question => "Answer question",
                First::Review => "Review candidate",
                First::Drafting | First::Critiquing | First::Revising => "Inspect worker",
                _ => "Inspect task",
            }
            .into()
        }
    }
    fn render_menu(&self, frame: &mut Frame<'_>, editor: Rect, base: Style, accent: Color) {
        let list = self.matches();
        let width = self.screen_width.saturating_sub(4).min(84);
        let visible = list.len().min(6);
        let description_rows = if width < 68 { 3 } else { 2 };
        let height = (visible + 3 + description_rows).max(5) as u16;
        let y = editor.y.saturating_sub(height);
        let rect = Rect::new(2, y, width, height);
        frame.render_widget(Clear, rect);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title("/ commands · ↑↓ choose · Enter act · Tab complete · Esc close")
                .style(base),
            rect,
        );
        let start = self.menu_index.saturating_sub(visible.saturating_sub(1));
        for (row_index, item) in list.iter().skip(start).take(visible).enumerate() {
            let i = start + row_index;
            let reason = self.enabled_reason(item.name);
            let row = format!(
                "{} /{:<13} {}",
                if i == self.menu_index { ">" } else { " " },
                item.name,
                if reason.is_some() {
                    "unavailable"
                } else {
                    item.shortcut
                }
            );
            Self::draw_line(
                frame,
                Rect::new(rect.x + 1, rect.y + 1 + row_index as u16, width - 2, 1),
                row,
                if i == self.menu_index {
                    base.fg(accent).add_modifier(Modifier::BOLD)
                } else {
                    base
                },
            );
        }
        if list.is_empty() {
            Self::draw_line(
                frame,
                Rect::new(rect.x + 1, rect.y + 1, width - 2, 1),
                "No command match · Enter offers Send as message",
                base,
            );
        }
        if let Some(selected) = list.get(self.menu_index) {
            let explanation = match self.enabled_reason(selected.name) {
                Some(reason) => format!("{} · Unavailable: {reason}", selected.description),
                None => selected.description.to_owned(),
            };
            frame.render_widget(
                Paragraph::new(explanation)
                    .wrap(Wrap { trim: false })
                    .style(base),
                Rect::new(
                    rect.x + 2,
                    rect.y + height - description_rows as u16 - 1,
                    width.saturating_sub(4),
                    description_rows as u16,
                ),
            );
        }
    }
    fn input_history_body(&self) -> (String, String) {
        let mut body = "Project-local composer history: sent/attempted messages and typed slash commands. Recall only fills the composer; Enter there remains explicit. Down past the newest entry restores your draft.\n\n".to_owned();
        let start = self.history_index.saturating_sub(11);
        for index in start..(start + 12).min(self.input_history.len()) {
            let entry = self.input_history.recent(index).unwrap();
            let preview: String = entry.text.replace('\n', " ⏎ ").chars().take(200).collect();
            body.push_str(&format!("{} {}{}\n", if index == self.history_index { "›" } else { " " },
                preview, if entry.text.chars().count() > 200 { "…" } else { "" }));
        }
        if self.input_history.is_empty() { body.push_str("No command history yet.\n"); }
        body.push_str("\nUp/Down select · Enter recall · Esc back. Chat scrolls with PageUp/PageDown.");
        ("Command history".into(), body)
    }
    fn queue_body(&self)->(String,String){
        let rows=self.queue_rows();let selected=self.queue_index.min(rows.len().saturating_sub(1));
        let start=selected.saturating_sub(1).min(rows.len().saturating_sub(3));let end=(start+3).min(rows.len());
        let mut text="Up/Down select · E restore to edit · PageUp/PageDown scroll details.\nApplied guidance cannot be withdrawn. Held messages require explicit release.\n".to_owned();
        if rows.is_empty(){text.push_str("\nNo queued or held messages. Your unsent draft remains in the composer.");}
        else{
            text.push_str(&format!("Inputs {}–{} of {}\n",start+1,end,rows.len()));
            for(i,r)in rows.iter().enumerate().take(end).skip(start){
                let label=match r["status"].as_str(){Some("reserved")=>if r["interrupt_requested"]==true{"Stopping"}else{"Running"},Some("pending")=>"Steering pending",Some("delivering")=>"Steering entering context",Some("applied")=>"Steering applied",Some("held")=>if r["input_id"].is_string(){"Steering held"}else{"Follow-up held"},_=>"Follow-up queued"};
                let raw=r["text"].as_str().unwrap_or("");let preview=raw.replace('\n'," ⏎ ").chars().take(64).collect::<String>();
                text.push_str(&format!("{} {} · {}{}\n",if i==selected{"›"}else{" "},label,preview,if raw.chars().count()>64{"…"}else{""}));
            }
            text.push_str(&format!("\nSelected input preview (E restores complete pending text):\n{}",rows[selected]["text"].as_str().unwrap_or("")));
        }
        ("Conversation queue".into(),text)
    }
    fn status_body(&self)->(String,String){
        let rows=self.queue_rows();
        let usage=self.coordinator_activity_lines().iter().map(|l|l.spans.iter().map(|s|s.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n");
        ("Status · current project".into(),format!("Project: {}\nConnection: {}\nSession: {}\nTopic: {}\n{}\nForeground coordinator: {}\nSteering pending/applying: {} · follow-ups queued: {}\n\n{}\n\n{}\nF7 opens queued inputs; E restores a pending input for editing.\n/interrupt stops the foreground call · /pause stops new dispatch.\nCtrl-Q leaves immediately; background work stays with the service.",
            self.live.as_ref().map_or("scripted fixture",|l|l.project.as_str()),self.journal.connection,
            self.live.as_ref().map_or("simulated",|l|l.phase.as_str()),self.selected_title(),
            self.model_header_label().unwrap_or_else(||"Model: not configured".into()),
            self.foreground_coordinator().and_then(|r|r["turn_id"].as_str()).unwrap_or("idle"),
            rows.iter().filter(|r|r["subject_id"]==self.conversation_subject&&matches!(r["status"].as_str(),Some("pending"|"delivering"))).count(),
            rows.iter().filter(|r|r["subject_id"]==self.conversation_subject&&r["status"]=="queued").count(),
            if usage.is_empty(){"Usage not reported for this topic"}else{&usage},
            if self.live_steering_available(){"Enter steers an active coordinator; Alt+Enter queues a follow-up."}else{"This service queues messages for later turns; live steering requires its next idle update."}))
    }
    fn modal_body(&self) -> (String, String) {
        if self.modal==Modal::Help&&self.continuous_mode(){
            return ("Help · conversation controls".into(),format!("{}\nAlt+Enter queues a separate follow-up, including in F2 multiline mode. F7 opens the queue; arrows select, E withdraws/restores pending text to edit without sending. Applied guidance cannot be withdrawn. Interruption holds unconsumed input; reopening never replays it.\n\nUp/Down moves inside multiline input and recalls history at its first/last line. Ctrl-J or Shift+Enter adds a newline; F2 is the fallback (Enter adds a newline, Tab selects Send). Ctrl-R opens history; PageUp/PageDown scrolls chat. Paste stays literal.\n\nEscape/Ctrl-C clears nonempty main input; with empty input it interrupts the foreground coordinator. /interrupt does the same. Escape in a popup backs out one level. Ctrl-Q leaves immediately from any editor/overlay; empty-input Ctrl-D or /exit also leaves. Background work stays with the service. /pause stops new dispatch separately; /resume explicitly continues.\n\n/model and /thinking open selection; S saves, A reviews activation. /status shows current state and reported usage. F3 settings, F4 target, F6 tasks. Type / to discover commands; Tab completes. Explicit scope/policy/question actions keep their own approval boundaries. Esc returns to the draft.",if self.live_steering_available(){"Enter sends idle input or steers the foreground coordinator at its next agent boundary."}else{"Enter sends a conversation message. This service queues later turns until its next idle update adds live steering."}));
        }
        if let Some(body)=self.dialogue_modal().filter(|_|!self.development_mode()||matches!(self.modal,Modal::Queue|Modal::QuestionConfirm)){return body}
        if self.live.is_some() { return self.live_modal_body(); }
        let review_actions = if self.candidate_read() {
            "[Back] [Read candidate] [Approve demo candidate] [Reject with reason]"
        } else {
            "[Back] [Read candidate] · Decisions appear after reading"
        };
        let reject_line = if self.candidate_read() {
            format!("Reject reason: {}", self.reject_reason)
        } else {
            String::new()
        };
        match self.modal {
            Modal::Status=>self.status_body(),
            Modal::Help=>("Help · SIMULATED".into(),
                "Type normally to the coordinator. F4 changes target to New task. Enter sends; Shift+Enter adds a newline when your terminal reports it distinctly. F2 toggles multiline as a fallback, then Tab to [Send]. Bracketed paste is literal. / opens commands; F6 tasks, F3 settings, F8 next scripted scene, Ctrl-Q detach. Up/Down recalls command history in the composer; Down past newest restores your draft. Alt+Up/Down moves the multiline cursor. Ctrl-R opens history. PageUp/PageDown scrolls chat eight lines; Alt+PageUp/PageDown moves one line. Escape closes this view. [Back]".into()),
            Modal::Tasks=>("Tasks · current project only".into(),format!("{} Clarify Decision Log choices · {}\n{} Manual walkthrough · {}\n\nUp/Down select. Enter returns to conversation. Tab/Escape back.\nWorkers remain labelled fake in this demonstration.",
                if self.journal.selected_task==0{">"}else{" "},self.first_short(),
                if self.journal.selected_task==1{">"}else{" "},self.second_short())),
            Modal::Scope=>("Review scope · SIMULATED".into(),format!("Task: {}\nPurpose: {}\nDeliverable: Project Decision Log specification addendum / manual walkthrough. No browser app build.\n\nSelected context: approved decision-log-01 artifact `revision-2.md` (source digest in Inspect). Editing policy B: free in-place editing; history is not kept. Open: alternatives minimum, supersession correction, cross-tab conflict. No unrelated project content.\n\nPrimary coordinator: scripted. Workers: fake author and fake critic. Limits: 5 worker calls and 2 critiques per task; {} used. No token/cost estimate.\n\nRevising this proposal is unavailable in the demo. Cancel targets this task.\n[Back] [Authorize simulated work] [Cancel task]  Selected: {}\nOpening this panel does not authorize.",
                self.selected_title(),if self.journal.selected_task==0{"Clarify remaining choices while preserving accepted editing policy"}else{"Prepare manual author walkthrough after addendum"},
                if self.journal.selected_task==0{self.journal.first_calls}else{self.journal.second_calls},
                match self.modal_action{1=>"Authorize",2=>"Cancel",_=>"Back"})),
            Modal::Consultation=>("Consultation unavailable".into(),"The scripted evaluator has no consultation gate. Use the installed mock service for this check.".into()),
            Modal::Steering=>("Steering unavailable".into(),"The scripted evaluator has no boundary-applied steering gate.".into()),
            Modal::Question if self.journal.first != First::Question => ("Question unavailable".into(), "No question is pending for the selected task. Inspect its current state.".into()),
            Modal::Question=>("Your answer needed · SIMULATED".into(),format!("Task: Clarify Decision Log choices\nMay a new decision have zero alternatives?\nWhy now: the approved specification leaves this validation rule open. Editing policy B is already accepted.\n\n{} 1. Allow none; display ‘No alternatives recorded’\n{} 2. Require at least one nonblank alternative\n{} 3. Other answer: {}\n\nAfter saving: the scripted author continues using your answer. Saving consumes no worker call. Free-form Other text is saved and quoted without interpretation.\nDelivery: {}\n\n[Back] [Save answer]  Selected: {}. Tab selects Save; Enter then records once.",
                if self.question_choice==Some(0){"(*)"}else{"( )"},if self.question_choice==Some(1){"(*)"}else{"( )"},
                if self.question_choice==Some(2){"(*)"}else{"( )"},self.question_text,self.journal.answer_delivery,
                if self.modal_action==1{"Save"}else{"Back"})),
            Modal::Review=>("Final review · SIMULATED".into(),format!("Candidate: scripted Decision Log addendum. Exact bytes in [Read candidate].\nSource approval remains intact; this is a separate demo journal.\nUnresolved: C1-3/C2-2 supersession correction; C2-3 cross-tab behavior. A free-form Other answer also leaves alternatives unresolved.\nThis revision received no further independent critique. Specification approval does not verify a working browser app.\n\nReview state: {}. Digest available in Inspect.\n\n{review_actions}\nRequest changes: unavailable; no extra call or rejection will be inferred.\n{}\nSelected action: {}. Approval binds the viewed exact candidate.",
                if self.journal.reviewed_sha256==self.journal.candidate_sha256{"candidate opened"}else{"not opened"},
                reject_line,match self.modal_action{1=>"Read",2=>"Approve",3=>"Reject",_=>"Back"})),
            Modal::Artifact=>("Candidate · exact scripted bytes".into(),format!("{}\n\nSHA-256: {}\n[Back to review]  Reading this marks this exact candidate as opened. No Vim/process is launched.",
                self.journal.candidate.as_deref().unwrap_or("No candidate"),self.journal.candidate_sha256.as_deref().unwrap_or("unavailable"))),
            Modal::Inspect=>("Selected task · evidence".into(),format!("{} · {}\nFirst task fake calls: {}/5; independent critiques: {}/2. Second task fake calls: {}/5.\nCoordinator fixture calls: {}/2 (illustrative, not a model bill).\nLatest journal event: #{latest_seq} · task key: {task_key}\nAnswer: {} · delivery: {}\nCandidate digest: {}\nInherited source SHA-256: {}\nContext: only the approved Project Decision Log specification, accepted editing decision B and five open finding references. No cross-project import granted.\n\nSecond task: {}. Existing fake receipts/counters remain; uncertain execution has no retry action.\n[Back]",
                self.selected_title(),self.selected_status(),self.journal.first_calls,if self.journal.first_calls>=2{1}else{0},
                self.journal.second_calls,self.journal.coordinator_calls,self.journal.answer.as_deref().unwrap_or("none"),
                self.journal.answer_delivery,self.journal.candidate_sha256.as_deref().unwrap_or("none"),SOURCE_HASH,self.second_short(),
                latest_seq=self.journal.next_seq.saturating_sub(1), task_key=if self.journal.selected_task==0 {"decision-log-addendum"} else {"manual-walkthrough"})),
            Modal::Worker=>("Fake worker preview · SIMULATED".into(),format!("{}\nRole: {} · provider/model: fake/deterministic-fixture-v1\nStage: {} · run: scripted-decision-log\n\n[preview] Scripted output retained in the local demo journal.\n[preview] No child process, Herdr worker pane or model session exists here.\n[preview] Fake calls: {}/5.\n\n[Back to Waywr]",
                self.selected_title(),if self.journal.first==First::Critiquing{"critic"}else{"author"},self.selected_status(),
                if self.journal.selected_task==0{self.journal.first_calls}else{self.journal.second_calls})),
            Modal::Settings=>("Coordinator settings · illustrative only".into(),format!("ACTIVE: scripted/decision-log-v1 · no model calls.\n\nPrimary coordinator example: Claude / sonnet; medium effort where that provider supports it. This is an illustration, not discovery or selection.\nWorker role profile examples: author Claude / sonnet / medium; independent critic Codex / gpt-6.1-sol / high. Neither runs here.\n\nFuture fallback policy: unavailable. No automatic provider switch. Limit reached: stop with an explicit state; a future switch would require a recorded decision and compatible settings. No universal effort levels are implied.\n\nLocal theme: {} (t cycles dark/light/mono). Effects: {} (e cycles subtle/reduced/off). These keys change appearance only.\n[Back]",
                self.journal.theme,self.journal.effects)),
            Modal::Target=>("Choose composer target · SIMULATED".into(),format!("{} Coordinator conversation — scripted response; no automatic task for every message.\n{} New task request — a distinct task proposal, still needs authorization.\n{} Steer selected work — unavailable in this demo.\n\nUp/Down select, Enter choose, Esc keeps the current target. No selection grants execution.",
                if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "},if self.modal_action==2{">"}else{" "})),
            Modal::Detach=>("Detach interface?".into(),"The scripted journal and draft will be saved. Real work is not running in this prototype.\n[Stay] [Detach]  Tab selects. Detach does not cancel or stop supervision in the production design.".into()),
            Modal::Pause=>("Pause project dispatch?".into(),"This action is available in the installed service, not the scripted evaluator.\n[Back]".into()),
            Modal::Cancel=>("Cancel selected task?".into(),format!("Target: {}. This changes only this task's isolated demo status and preserves fake call evidence.\n[Back] [Cancel selected demo task]  Tab selects; no worker process is signalled.",self.selected_title())),
            Modal::UnknownCommand=>("Unknown slash command".into(),format!("{}\nNo action was run.\n[Edit draft] [Send as literal coordinator message]  Tab selects.",self.composer.text())),
            Modal::History => self.input_history_body(),
            Modal::ScopeEditor=>("Edit saved scope".into(),"Use Tab to select a field, edit, save or discard. No model call.".into()),
            Modal::Queue|Modal::QuestionConfirm=>("Unavailable".into(),"This bound release does not expose continuous conversation controls.".into()),
            Modal::None=>(String::new(),String::new()),
        }
    }
    fn live_modal_body(&self) -> (String,String) {
        if self.modal==Modal::Status{return self.status_body();}
        if self.provider_settings(){return self.provider_body();}
        if self.development_mode(){let dev=self.development_view();match self.modal {
            Modal::Cancel if self.development_workflow().is_some()&& !self.queue_rows().iter().any(|r|r["status"]=="reserved")=>return ("Cancel development work?".into(),"[Back] [Cancel workflow] · Fence new assignments and request cancellation of this project's active worker/coordinator attempts. Preserve task commits, dirty worktrees, receipts and request history. Cancellation is recorded after settled execution; absent processes alone prove nothing.".into()),
            Modal::Settings=>return ("Development agents and operating policy".into(),format!("Codex · coordinator and workers: gpt-6.1-sol / medium\nConcurrent task worktrees; serialized integration.\nReadiness: {}\nCapacity waiting; no configured fallback. No mandatory invocation, repair or task-time ceiling. Provider cost is unknown unless reported.\n\nProject policy:\n{}",self.chat().map_or("unavailable",|v|v["status"].as_str().unwrap_or("unknown")),self.current_scope().map_or(String::new(),|h|serde_json::to_string_pretty(&h["operating_policy"]).unwrap_or_default()))),
            Modal::Inspect=>{
                let mut text=String::new();
                for w in dev["workflows"].as_array().into_iter().flatten(){if let Some(path)=w["integration_checkout"]["directory"].as_str(){text.push_str(&format!("Combined branch: {}\nCommit: {}\nCombined checkout: {}\nUse this directory for preview; main advances only after delivery.\n",Self::safe(w["result_branch"].as_str().or(w["integration_branch"].as_str()).unwrap_or("pending")),Self::safe(w["head"].as_str().unwrap_or("unknown")),Self::safe(path)));}}
                for d in dev["context_diagnostics"].as_array().into_iter().flatten().take(4){text.push_str(&format!("\nContext · {} · profile {} · {}/{}\n  Packet: {} bytes · soft target {} · application memory {}\n  Estimated input tokens: {} (heuristic; exact tokenizer/native capacity unknown)\n  Instructions: {} bytes · schema: {} bytes\n  Headroom: {}\n  Section bytes: {}\n  Omissions: {}\n  Read-only retrieval: {}\n  Diagnostics: {}\n",d["role"],d["profile_id"],d["provider"],d["model"],d["packet_bytes"],d["working_target_bytes"],d["application_memory_limit_bytes"],d["token_estimate"]["value"],d["instructions_bytes"],d["schema_bytes"],d["headroom"],d["section_bytes"],d["omissions"],d["retrieval_available"],d["diagnostics_file"]));}
                let mut records=dev.clone();if let Some(o)=records.as_object_mut(){o.remove("context_diagnostics");}
                text.push_str("\nDevelopment records and readable history paths:\n");text.push_str(&serde_json::to_string_pretty(&records).unwrap_or_default());
                return ("Development · context, commits, checks and request history".into(),text);
            },
            Modal::Help=>return ("Development workflow".into(),"Discuss the project normally. Up/Down recalls command history in the composer; Down past newest restores your draft. Alt+Up/Down moves the multiline cursor. Ctrl-R opens history. PageUp/PageDown scrolls chat eight lines; Alt+PageUp/PageDown moves one line. /plan opens the current topic’s saved scope and operating policy for one explicit approval. Workers then edit isolated task worktrees; the coordinator reviews frozen commits and the service integrates and runs checks. Submitted work is distinct from accepted delivery. F6 shows tasks; /inspect shows obligations, integration evidence and absolute readable log/summary paths. Conversation stays available. /detach keeps work running; /exit leaves immediately; /pause stops new dispatch while calls settle. Resume uses waywr resume. Unexpected dirty or ambiguous execution is preserved for reconciliation.".into()),
            _=>{}
        }}
        if self.scope_mode(){match self.modal{
            Modal::Help=>("Help · scope discussion".into(),"Enter sends ordinary text to the coordinator; Shift+Enter inserts a newline, F2 is the multiline fallback. Paste stays literal. Empty-input Enter opens the visible next action. Questions save first, then Continue explicitly spends a call. Review scope offers local edits, revision requests and a separate scope authorization. No workers or consultation can run. /plan opens the saved scope; /inspect shows records and /output shows provisional coordinator text. Tab selects visible actions. /detach closes the UI; /exit leaves immediately; /pause stops dispatch; waywr resume reconciles. Mock transport is a scripted regression scenario.".into()),
            Modal::Scope=>self.scope_body(),
            Modal::ScopeEditor=>{
                let field=Self::SCOPE_FIELDS[self.scope_field];let conflict=self.scope_edit.as_ref().is_some_and(|e|self.current_scope().is_none_or(|h|e["payload"]["base"]!=h["base"]||e["payload"]["decision_revision"]!=h["decision_revision"]||e["payload"]["authorization_command_id"]!=h["authorization_command_id"]));
                (format!("Local scope edit · {field}"),format!("{}\nTab: field / text / Save / Back / Discard. Arrows choose the field when Field is focused. Lists: one item per line. Enter inserts a line in Text; paste stays literal. Save requires a rationale (select its field). Escape retains the edit. No model call.\n",if conflict{"Saved scope changed; this retained edit will be refused. Inspect it, then explicitly discard/reopen to use the latest revision."}else{"Editing the displayed saved revision"}))
            },
            Modal::Question if self.pending_conversation_question().is_none()=>{
                if let Some(q)=self.saved_scope_answer(){return ("Answer saved".into(),format!("Question: {}\nSaved answer: {}\n\nNo call was spent saving the answer. Continue submits it once to the coordinator and uses one remaining call. Opening this panel launches nothing.",q["prompt"].as_str().unwrap_or(""),q["answer"].as_str().unwrap_or("")));}
                ("Question unavailable".into(),"No open question or saved continuation is pending.".into())
            },
            Modal::Settings=>{let chat=self.chat();let limit=chat.map_or(Value::Null,|v|v["profile"]["call_limit"].clone());("Coordinator settings · scope only".into(),format!("Transport: {}\nProvider: Claude Code\nModel: claude-sonnet-5-5 · medium effort\nReadiness: {}\n{}\n\nMaximum {limit} coordinator invocations; zero workers/task calls and consultation. No retry or fallback. Scope decisions never launch work.\nContext: current proposal, linked answers and bounded discussion. Tools/MCP/skills/hooks/history disabled; existing login stays in place for native mode.\nOnly a separately authorized live trial may enable native allowance. Mock transport uses the pinned scripted fixture. Opening settings costs nothing.\n\nTheme: {} (t) · Effects: {} (e)",if chat.is_some_and(|v|v["profile"]["mode"]=="mock-cli"){"MOCK CLI STREAM · scripted regression scenario"}else{"Native · separately authorized"},chat.map_or("blocked",|v|v["status"].as_str().unwrap_or("blocked")),chat.and_then(|v|v["reason"].as_str()).unwrap_or(""),self.journal.theme,self.journal.effects))},
            Modal::Inspect if self.current_scope().is_some()=>{let scope=self.current_scope().unwrap();("Saved scope · project evidence".into(),format!("Scope revision and decisions:\n{}\n\nExecution unavailable; no run or worker admitted. Conversation calls and usage references:\n{}",serde_json::to_string_pretty(scope).unwrap_or_default(),serde_json::to_string_pretty(&self.live.as_ref().unwrap().conversation.as_ref().unwrap()["turns"]).unwrap_or_default()))},
            Modal::Target=>("Input destination".into(),format!("{} Discuss selected scope (or current conversation when none exists)\n{} New topic / proposal discussion — no selected scope\n\nUp/Down select, Enter chooses. Ordinary messages never authorize or launch workers.",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "})),
            _=>return self.live_modal_body_general(),
        }}else{self.live_modal_body_general()}
    }
    fn live_modal_body_general(&self)->(String,String){
        let live=self.live.as_ref().expect("live mode");let selected=live.selected(self.journal.selected_task);let title=live.title(self.journal.selected_task);let status=live.status(self.journal.selected_task);
        let detail=live.detail.as_ref().filter(|v|v["task"]["task_id"]==selected.map_or(Value::Null,|v|v["task_id"].clone()));let pending=detail.map(|v|&v["pending"]);
        if let Some(chat)=self.chat(){match self.modal{
            Modal::Settings=>return ("Coordinator settings · conversation only".into(),format!("Provider: Claude Code\nModel: {}\nEffort: {}\nReadiness: {}\n{}\n\nMaximum: three coordinator invocations, shared across detach/resume. No workers, consultation, fallback or retries.\nSelected context: up to six recent messages plus the current project's bounded handoff.\nTools/MCP/skills/hooks/provider history disabled. Existing CLI login stays in place.\n\nAuthorize only after the separate operator live-call approval. Opening settings launches nothing.\n\nTheme: {} (t) · Effects: {} (e)",chat["profile"]["model"].as_str().unwrap_or("missing"),chat["profile"]["effort"].as_str().unwrap_or("missing"),chat["status"].as_str().unwrap_or("blocked"),chat["reason"].as_str().unwrap_or(""),self.journal.theme,self.journal.effects)),
            Modal::Inspect=>return ("Coordinator conversation · project evidence".into(),format!("Readiness: {}\n{}\nProvider/model: Claude / {}\nEffort: {}\nConversation allowance: {}/{} held or charged\n\nRecent calls and execution-record locations:\n{}\n\nEach operation's immutable attempt retains receipt.json, packet.json and native-events.jsonl. Reported usage/client cost estimates stay in native-events.jsonl; a validated completion also saves runtime-result.json. Partial text stays provisional. No worker execution was admitted; consultation/execution are unavailable.\n\nContext is the selected current-project handoff and bounded prior messages; no unrelated project is discovered.",chat["status"].as_str().unwrap_or("blocked"),chat["reason"].as_str().unwrap_or(""),chat["profile"]["model"].as_str().unwrap_or("unconfigured"),chat["profile"]["effort"].as_str().unwrap_or("unconfigured"),live.conversation.as_ref().map_or(Value::Null,|v|v["budgets"]["conversation"]["charged"].clone()),live.conversation.as_ref().map_or(Value::Null,|v|v["budgets"]["conversation"]["limit"].clone()),live.conversation.as_ref().map_or(String::new(),|v|serde_json::to_string_pretty(&v["turns"]).unwrap_or_default()))),
            Modal::Cancel=>return ("Interrupt coordinator call?".into(),"[Back] [Interrupt call] · Stops only this project's active call. Partial output and held budget remain. An interrupted/uncertain trial will not replay automatically.".into()),
            Modal::Worker if !self.development_mode()=>return ("Coordinator output".into(),format!("{}\n\nCompletion/receipt diagnostics remain available in /inspect.",live.conversation.as_ref().and_then(|v|v["provisional"]["text"].as_str()).unwrap_or("No provisional output; completed replies remain in the conversation."))),
            Modal::Target=>return ("Input destination".into(),"Coordinator conversation is the only destination in this increment. Workers, consultation and active-task steering are deferred. Escape returns to your draft.".into()),
            _=>{}
        }}
        match self.modal {
            Modal::Help=>("Help · project service".into(),format!("Enter sends {}; Shift+Enter inserts a newline. F2 enables multiline if the terminal cannot report Shift+Enter. Paste is literal. Type / for actions; Tab completes, Enter invokes. F6 tasks, F4 target, PageUp/PageDown scrolls chat eight lines; Alt+PageUp/PageDown one line. /output opens live or retained worker output; Left/Right chooses a stage. /detach closes only this interface; /exit leaves immediately; /pause stops new dispatch while calls settle separately. {}",
                if live.conversation.is_some(){"a coordinator message"}else{"a new task request"},
                if self.chat().is_some(){"Coordinator readiness and allowance are in F3 Settings. /cancel interrupts this project's active conversation call."}else if live.conversation.is_some(){"Coordinator responses are deterministic mock fixtures, not authenticated model interpretation."}else{"This service uses a deterministic fake coordinator; open-ended conversation is unavailable."})),
            Modal::Tasks=>("Tasks · current project".into(),format!("{}\n\nUp/Down select · Enter returns · Esc closes",self.tasks_text())),
            Modal::Scope=>{
                let Some(result)=live.plan.as_ref() else {return ("Scope unavailable".into(),"Request /plan on a proposed task. No authorization was sent.".into());};
                let plan=&result["plan"];
                ("Review scope · simulated workers".into(),format!("Task: {title}\nBrief: {}\n\nProject: {}\nProfile: {}\nContext selection: {}\nExplicit imports: {}\nAuthor: {}\nCritic: {}\nMaximum worker calls: {} · critiques: {}\n\n[Back] [Authorize] · Opening this panel does not authorize.\nPlan digest and exact revisions are kept by the service.",
                    plan["brief"].as_str().unwrap_or("unknown"),plan["project_key"].as_str().unwrap_or("unknown"),
                    plan["profile"].as_str().unwrap_or("unknown"),plan["context"].as_str().unwrap_or("none"),
                    plan["cross_imports"].as_array().map_or(0,Vec::len),
                    plan["config"]["roles"]["author"],plan["config"]["roles"]["critic"],
                    plan["max_worker_invocations"],plan["review_limit"]))
            },
            Modal::Question=>{
                if let Some(p)=self.pending_conversation_question(){
                    let mut text=format!("Coordinator question: {}\nWhy: {}\n\n",p["prompt"].as_str().unwrap_or("A decision is requested"),p["scope"]["rationale"].as_str().unwrap_or("Clarifies the saved scope"));
                    let options=p["options"].as_array().cloned().unwrap_or_default();
                    for (i,option) in options.iter().enumerate(){
                        text.push_str(&format!("{} {}. {}\n",if self.question_choice==Some(i){"(*)"}else{"( )"},i+1,option.as_str().unwrap_or("")));
                    }
                    text.push_str(&format!("{} {}. Other answer: {}\n\n[Back] [Save answer] · The answer is saved once; continuing with the coordinator uses a remaining call.",
                        if self.question_choice==Some(options.len()){ "(*)" }else{"( )"},options.len()+1,self.question_text));
                    return (if self.scope_mode(){"Your answer needed"}else{"Your answer needed · simulated coordinator"}.into(),text);
                }
                let Some(p)=self.pending_choice() else {return ("Question unavailable".into(),"No choice question is pending for the selected task. Open /inspect to see its current state.".into());};
                let mut text=format!("Task: {title}\n{}\n\n",p["question"]["question"].as_str().unwrap_or("A decision is requested"));
                if let Some(options)=p["question"]["options"].as_array() {
                    for (i,option) in options.iter().enumerate() {
                        text.push_str(&format!("{} {}. {}\n",if self.question_choice==Some(i){"(*)"}else{"( )"},i+1,option.as_str().unwrap_or("")));
                    }
                }
                text.push_str("\n[Back] [Save answer] · This workflow accepts only its listed choices. Free-form answers require a backend contract. Answering does not consume a worker call.");
                ("Your answer needed · simulated workflow".into(),text)
            },
            Modal::Consultation=>{
                let Some(action)=self.pending_consultation() else{return ("Consultation unavailable".into(),"No second-agent request is pending.".into());};
                ("Review consultation · simulated".into(),format!("Requested opinion: {}\n\nOne mock consultation call remains: {}. This advice cannot authorize work or edit files.\n\n[Back] [Authorize one consultation] · Opening this panel does not spend a call.",
                    action["body"]["payload"]["question"].as_str().unwrap_or("Selected task opinion"),
                    live.conversation.as_ref().map_or(0,|v|v["budgets"]["consultation"]["remaining"].as_i64().unwrap_or(0))))
            },
            Modal::Steering=>{
                let Some(action)=self.pending_steering() else{return ("Steering unavailable".into(),"No suggestion is pending for the selected task.".into());};
                ("Review steering · simulated".into(),format!("Selected task: {title}\nSuggested change: {}\n\nApplying this before authorization retains the original proposal in history, closes it, and creates a linked unadmitted proposal. Any displayed scope for the original becomes stale. It spends no model call and authorizes no worker.\n\n[Back] [Apply steering]",
                    action["body"]["payload"]["text"].as_str().unwrap_or("Unavailable")))
            },
            Modal::Review=>{
                let Some(p)=pending.filter(|v|v["kind"]=="final-review") else {return ("Review unavailable".into(),"No final review is pending for the selected task. Refresh /inspect.".into());};
                let hash=p["artifact_sha256"].as_str().unwrap_or("unknown");
                let reviewed=self.candidate_read();
                let actions=if reviewed {"[Back] [Read candidate] [Approve exact candidate] [Reject]"}
                    else {"[Back] [Read candidate] · Decisions appear after reading"};
                let reject_line=if reviewed {format!("Reject reason: {}",self.reject_reason)}
                    else {String::new()};
                if detail.is_some_and(|v|v["run"]["profile"]=="static-files-v1"){
                    let findings=detail.and_then(|v|v["findings"].as_array()).map(|items|items.iter()
                        .filter(|finding|finding["status"]!="addressed")
                        .map(|finding|format!("• {}: {}",finding["id"].as_str().unwrap_or("finding"),
                            finding["requested_change"].as_str().unwrap_or("Inspect evidence")))
                        .collect::<Vec<_>>().join("\n")).unwrap_or_default();
                    let last=p["last_reviewed"]["sha256"].as_str().unwrap_or("no matching review");
                    return ("Final review · simulated file workflow".into(),format!(
                        "Task: {title}\nNext: {}\nCandidate bundle: {}\nExact manifest SHA-256: {hash}\nOpened and verified: {}\nLast independent mock review covered manifest: {last}\n\nUnresolved findings:\n{}\n\n{actions}\n{}\nThis is a simulated file bundle. Approval binds these exact files; a working browser application and native-agent execution have not been verified.",
                        if reviewed{"Choose Approve or Reject after reviewing this exact bundle."}else{"Select Read candidate, inspect every page, then return here to decide."},
                        detail.and_then(|v|v["artifact_path"].as_str()).unwrap_or("unavailable"),
                        if reviewed{"yes"}else{"no"},if findings.is_empty(){"None recorded as unresolved"}else{&findings},reject_line));
                }
                let findings=detail.and_then(|v|v["findings"].as_object())
                    .map(|items|items.iter().filter(|(_,value)|value["status"]=="open")
                        .map(|(id,value)|format!("• {id}: {}",value["requested_change"].as_str()
                            .or_else(||value["evidence"].as_str()).unwrap_or("Inspect finding evidence")))
                        .collect::<Vec<_>>().join("\n")).unwrap_or_default();
                ("Final review · simulated workflow".into(),format!("Task: {title}\nNext: {}\nCandidate: {}\nExact SHA-256: {hash}\nOpened and verified: {}\nLast independent review: {}\n\nUnresolved findings:\n{}\n\n{actions}\n{}\nApproval accepts this specification artifact only; it does not verify an application. The latest revision may not have another independent critique.",
                    if reviewed{"Choose Approve or Reject after reviewing this exact candidate."}else{"Select Read candidate, inspect every page, then return here to decide."},
                    detail.and_then(|v|v["artifact_path"].as_str()).unwrap_or("unavailable"),if reviewed{"yes"}else{"no"},
                    p["last_reviewed"].as_str().unwrap_or("inspect run evidence"),
                    if findings.is_empty(){"None recorded as open"}else{&findings},reject_line))
            },
            Modal::Artifact=>(if let Some(name)=&live.preview_file{format!("Candidate file · {name}")}
                else{"Candidate · exact project artifact".into()},format!("{}\n\nSHA-256: {}\n{} · [Back to review]",
                live.artifact.as_deref().unwrap_or("Loading verified candidate page…"),
                live.detail.as_ref().and_then(|detail|detail["pending"]["artifact_sha256"].as_str()).unwrap_or("unavailable"),
                if live.preview_file.is_some(){"One file inspected. Read the full bundle before approval"}
                else if live.artifact_complete{"All pages received; exact candidate eligible for review"}
                    else{"Press n for next verified page; approval remains blocked"})),
            Modal::Inspect=>("Selected task · project evidence".into(),format!("Task: {title}\nStatus: {status}\nWorkflow run: {}\nTask ID: {}\n\nCoordinator execution:\n{}\n\nPending gate:\n{}\n\nCurrent artifact:\n{}\n\nConversation budgets/context:\n{}\n\nContext and grants are limited to this project and explicit task imports. Service records and receipts remain authoritative.",
                selected.and_then(|v|v["run_id"].as_str()).unwrap_or("not started"),
                selected.and_then(|v|v["task_id"].as_str()).unwrap_or("none"),
                detail.and_then(|v|v.get("coordinator")).map(|v|serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or("No coordinator call recorded".into()),
                pending.map(|v|serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or("No pending gate".into()),
                detail.and_then(|v|v["artifact_path"].as_str()).unwrap_or("none"),
                live.conversation.as_ref().map(|v|serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or("schema-3 task session".into()))),
            Modal::Worker=>{
                let item=live.operations.get(live.worker_index);
                let stage=item.and_then(|v|v["stage"].as_str()).unwrap_or("no execution yet");
                let state=if live.worker_terminal{"retained"}else{"live or awaiting output"};
                let limit=if live.worker_limit.is_some(){"\nOutput limit reached; execution stopped. Inspect the receipt."}else{""};
                ("Worker output · project execution".into(),format!("Task: {title}\nStage: {stage} · {state}\nLeft/Right choose stage · Up/Down line · PageUp/PageDown page · Esc returns\n\n{}{}",
                    if live.worker_output.is_empty(){"No published output yet."}else{&live.worker_output},limit))
            },
            Modal::Settings=>("Coordinator settings".into(),format!("ACTIVE: fake/deterministic-coordinator-v1. Author and critic roles use the project's frozen fake profile. No authenticated model calls are made by this session service.\n\nNative admission is closed after the Codex capability proof failed. Provider switching and fallback are unavailable. Explicit mock replacement advances the binding generation while preserving project history and budgets; it cannot replace an active call.\n\nTheme: {} (t cycles dark/light/mono). Effects: {} (e cycles subtle/reduced/off). These change presentation only.\n\n[Back] [Replace mock binding] · No model call is launched.",self.journal.theme,self.journal.effects)),
            Modal::Target=>("Choose input destination".into(),format!("{} Coordinator conversation — {}.\n{} New task request — {}.\n{} Steer selected task — {}.\n\nChoose with Up/Down, Enter. No selection grants execution.",
                if self.modal_action==0{">"}else{" "},
                if live.conversation.is_some(){"scripted mock discussion, no automatic task"}else{"unavailable in this bound schema-3 service"},
                if self.modal_action==1{">"}else{" "},
                if live.conversation.is_some(){"asks the mock coordinator for a proposal; authorization stays separate"}else{"deterministic proposal; separate authorization required"},
                if self.modal_action==2{">"}else{" "},
                if live.conversation.is_some(){"recorded suggestion for the selected task; no frozen input changes automatically"}else{"unavailable in this service"})),
            Modal::Detach=>("Detach interface?".into(),"[Stay] [Detach] · Authorized work continues under this project's supervisor. Draft and decisions remain in project state. No task is cancelled.".into()),
            Modal::Pause=>("Pause project dispatch?".into(),"[Stay] [Pause dispatch] · Stop new dispatch for this project. Already permitted calls may finish under supervision. Progress and decisions are saved continuously. A running call is not killed or retried. Resume with waywr resume.".into()),
            Modal::Cancel=>("Cancel selected task?".into(),format!("Task: {title}\n[Back] [Cancel task] · Only this project's selected task is targeted. The service decides whether cancellation is safe.")),
            Modal::UnknownCommand=>("Unknown slash command".into(),format!("{}\nNo action was run. [Edit draft] [Send as literal task request]",self.composer.text())),
            Modal::History => self.input_history_body(),
            Modal::ScopeEditor=>("Edit saved scope".into(),"Use Tab to select a field, edit, save or discard. No model call.".into()),
            Modal::Queue|Modal::QuestionConfirm=>("Unavailable".into(),"Continuous conversation controls require the compatible release.".into()),
            Modal::None|Modal::Status=>(String::new(),String::new()),
        }
    }
    fn render_modal(
        &mut self,
        frame: &mut Frame<'_>,
        screen: Rect,
        base: Style,
        accent: Color,
        _attention: Color,
    ) {
        if self.provider_settings(){self.render_provider_modal(frame,screen,base,accent);return;}
        let (title, body) = self.modal_body();
        let width = screen.width.saturating_sub(4).min(104);
        let height = screen.height.saturating_sub(4).min(if screen.height >= 42 {
            34
        } else {
            screen.height - 2
        });
        let rect = Rect::new(
            (screen.width - width) / 2,
            (screen.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(accent))
            .style(base);
        frame.render_widget(block, rect);
        let inner = Rect::new(
            rect.x + 1,
            rect.y + 1,
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        );
        let footer_h = if self.scope_mode()&&matches!(self.modal,Modal::Scope|Modal::ScopeEditor){4.min(inner.height)}else{2.min(inner.height)};
        let content = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner.height.saturating_sub(footer_h),
        );
        let paragraph = Paragraph::new(Text::from(markdown_lines(
                &Self::safe(&body),
                content.width,
                Tone::for_theme(&self.journal.theme),
                false,
            )))
            .wrap(Wrap { trim: false })
            .style(base);
        self.modal_scroll_limit = paragraph.line_count(content.width)
            .saturating_sub(content.height as usize)
            .min(u16::MAX as usize) as u16;
        self.modal_scroll = self.modal_scroll.min(self.modal_scroll_limit);
        if self.modal==Modal::ScopeEditor{
            let hint_h=4.min(content.height);frame.render_widget(paragraph.clone().scroll((self.modal_scroll,0)),Rect::new(content.x,content.y,content.width,hint_h));
            let edit_rect=Rect::new(content.x,content.y+hint_h,content.width,content.height.saturating_sub(hint_h));
            self.scope_editor.area.set_style(base);self.scope_editor.area.set_cursor_style(if self.modal_action==1{base.add_modifier(Modifier::REVERSED)}else{base});frame.render_widget(&self.scope_editor.area,edit_rect);
        }else{frame.render_widget(paragraph.scroll((self.modal_scroll, 0)), content);}
        let actions = if self.modal == Modal::History {
            "[Recall entry] · Up/Down select · Enter recall · Esc back".into()
        } else if self.live.is_some() { match self.modal {
            Modal::Queue=>{let rows=self.queue_rows();let selected=rows.get(self.queue_index);let mut controls=vec!["Back"];if selected.is_some_and(Self::queue_editable){controls.push("Withdraw message")}if selected.is_some_and(|r|r["status"]=="held"&&!r["input_id"].is_string())&&self.can_release_held(){controls.push("Continue selected held message")}format!("{} · E restore to edit",controls.iter().enumerate().map(|(i,n)|format!("{} [{n}]",if i==self.modal_action{"›"}else{" "})).collect::<Vec<_>>().join(" "))},
            Modal::QuestionConfirm=>{let controls=self.confirmation_controls();controls.iter().enumerate().map(|(i,n)|format!("{} [{n}]",if i==self.modal_action{"›"}else{" "})).collect::<Vec<_>>().join(" ")},
            Modal::Scope if self.scope_mode()=>{
                let actions=self.scope_actions();
                let primary=format!("{} [{}] · Enter activates; ←/→ or Tab selects",if self.modal_action==0{"›"}else{" "},actions[0].1);
                let secondary=actions.iter().enumerate().skip(1).map(|(i,(_,name))|format!("{} [{name}]",if i==self.modal_action{"›"}else{" "})).collect::<Vec<_>>().join(" ");
                format!("{primary}\n{secondary}")
            },
            Modal::ScopeEditor=>["Field","Text","Save revision","Back","Discard edit"].iter().enumerate().map(|(i,n)|format!("{} [{n}]",if i==self.modal_action{"›"}else{" "})).collect::<Vec<_>>().join(" "),
            Modal::Question if self.saved_scope_answer().is_some()&&self.pending_conversation_question().is_none()=>format!("{} [Back] {} [Continue · uses 1 call]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Scope=>format!("{} [Back] {} [Authorize]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Question if self.pending_choice().is_some() || self.pending_conversation_question().is_some()=>format!("{} [Back] {} [Save answer]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Question=>"[Back] · No question pending".into(),
            Modal::Consultation if self.pending_consultation().is_some()=>format!("{} [Back] {} [Authorize consultation]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Steering if self.pending_steering().is_some()=>format!("{} [Back] {} [Apply steering]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Settings if self.provider_settings()=>"1–6 section · S save · L reload · R local refresh · Esc back".into(),
            Modal::Settings if self.native_dialogue_blocked()=>"[Back] · Native admission unavailable".into(),
            Modal::Settings if self.chat().is_some()=>format!("{} [Back] {} [{}]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "},if self.chat().is_some_and(|v|v["authorized"]==true){"Allowance already saved"}else if self.scope_mode(){"Authorize six conversation calls"}else{"Authorize three conversation calls"}),
            Modal::Settings if self.live.as_ref().is_some_and(|v|v.conversation.is_some())=>format!("{} [Back] {} [Replace mock binding]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Review if self.candidate_read()=>format!("{} [Back] {} [Read] {} [Approve] {} [Reject]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "},if self.modal_action==2{">"}else{" "},if self.modal_action==3{">"}else{" "}),
            Modal::Review=>format!("{} [Back] {} [Read candidate] · Decisions after reading",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Pause=>format!("{} [Stay] {} [Pause dispatch]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Detach=>format!("{} [Stay] {} [Detach]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            Modal::Cancel=>format!("{} [Back] {} [{}]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "},if self.chat().is_some(){"Interrupt call"}else{"Cancel task"}),
            Modal::UnknownCommand=>format!("{} [Edit draft] {} [Send as literal message]",if self.modal_action==0{">"}else{" "},if self.modal_action==1{">"}else{" "}),
            _=>"Esc back · ↑/↓ line · PgUp/PgDn page · Home/End bounds".into(),
        }} else { match self.modal {
            Modal::Scope => format!(
                "{} [Back] {} [Authorize] {} [Cancel]",
                if self.modal_action == 0 { ">" } else { " " },
                if self.modal_action == 1 { ">" } else { " " },
                if self.modal_action == 2 { ">" } else { " " }
            ),
            Modal::Question if self.journal.first == First::Question => format!(
                "{} [Back] {} [Save answer]",
                if self.modal_action == 0 { ">" } else { " " },
                if self.modal_action == 1 { ">" } else { " " }
            ),
            Modal::Question => "[Back] · No question pending".into(),
            Modal::Review if self.candidate_read() => format!(
                "{} [Back] {} [Read] {} [Approve] {} [Reject]",
                if self.modal_action == 0 { ">" } else { " " },
                if self.modal_action == 1 { ">" } else { " " },
                if self.modal_action == 2 { ">" } else { " " },
                if self.modal_action == 3 { ">" } else { " " }
            ),
            Modal::Review => format!(
                "{} [Back] {} [Read candidate] · Decisions after reading",
                if self.modal_action == 0 { ">" } else { " " },
                if self.modal_action == 1 { ">" } else { " " }
            ),
            Modal::Detach | Modal::Pause | Modal::Cancel | Modal::UnknownCommand => format!(
                "{} [Back] {} [Confirm]",
                if self.modal_action == 0 { ">" } else { " " },
                if self.modal_action == 1 { ">" } else { " " }
            ),
            _ => "Esc back · ↑/↓ line · PgUp/PgDn page · Home/End bounds".into(),
        }};
        frame.render_widget(Paragraph::new(actions).wrap(Wrap{trim:false}).style(base.fg(accent).add_modifier(Modifier::BOLD)),Rect::new(inner.x,inner.y+inner.height.saturating_sub(footer_h),inner.width,footer_h.saturating_sub(1)));
        Self::draw_line(
            frame,
            Rect::new(
                inner.x,
                inner.y + inner.height.saturating_sub(1),
                inner.width,
                1,
            ),
            if self.chat().is_some(){"PROJECT SERVICE · conversation and decisions are durable"}else if self.live.is_some(){"SIMULATED SERVICE · actions go through durable project IPC"}else{"SIMULATED DEMO · actions affect this isolated journal only"},
            base,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ratatui::{backend::TestBackend, Terminal};
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    #[test]
    fn live_output_cursor_and_exact_candidate_review_are_scoped(){
        let mut app=App::new_live("fixture".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"session":{"phase":"running"},
            "tasks":[{"task_id":"task-a","status":"awaiting-input","title":"Decision log"}],
            "recent_events":[]}));
        app.apply_operation_list(&serde_json::json!({"task_id":"task-a","operations":[
            {"stage":"draft","attempt_id":"attempt-a","output_available":true}]}));
        let page=serde_json::json!({"task_id":"task-a","stage":"draft","page":{
            "format":"indexed-v1","cursor":{"format":"indexed-v1","attempt":"attempt-a",
                "sequence":2,"offset":0},"high_water":1,"terminal":true,"output_limit":null}});
        app.apply_worker_output(&page,"[stdout] Draft written\n");
        app.apply_worker_output(&page,"[stdout] Draft written\n");
        assert_eq!(app.live.as_ref().unwrap().worker_output.matches("Draft written").count(),1);
        assert!(app.worker_read_request().is_none());
        app.apply_worker_output(&serde_json::json!({"task_id":"task-b","stage":"draft",
            "page":page["page"]}),"wrong project");
        assert!(!app.live.as_ref().unwrap().worker_output.contains("wrong project"));
        app.apply_detail(serde_json::json!({"task":{"task_id":"task-a"},
            "pending":{"artifact_sha256":"hash-a","kind":"final-review"}}));
        app.apply_artifact_page(&serde_json::json!({"task_id":"task-a","sha256":"hash-a",
            "offset":0,"next_offset":4,"complete":false}),"part");
        assert!(app.live.as_ref().unwrap().reviewed_sha.is_none());
        app.apply_artifact_page(&serde_json::json!({"task_id":"task-a","sha256":"hash-b",
            "offset":4,"next_offset":8,"complete":true}),"wrong");
        assert!(app.live.as_ref().unwrap().reviewed_sha.is_none());
        app.apply_artifact_page(&serde_json::json!({"task_id":"task-a","sha256":"hash-a",
            "offset":4,"next_offset":8,"complete":true}),"ial");
        assert_eq!(app.live.as_ref().unwrap().artifact.as_deref(),Some("partial"));
        assert_eq!(app.live.as_ref().unwrap().reviewed_sha.as_deref(),Some("hash-a"));
        for (width,height) in [(80,24),(160,48)]{
            app.modal=Modal::Worker;
            let mut terminal=Terminal::new(TestBackend::new(width,height)).unwrap();
            terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
            let display=terminal.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();
            assert!(display.contains("Draft written"),"worker output missing at {width}x{height}");
        }
    }
    #[test]
    fn renders_supported_sizes_and_simulation_label() {
        for (w, h) in [(60, 18), (80, 24), (120, 36), (160, 48)] {
            let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal
                .draw(|frame| app.render(frame, Duration::ZERO))
                .unwrap();
            let screen = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(screen.contains("SIMULATED"), "{w}x{h}");
            assert!(screen.contains("Project Decision Log"), "{w}x{h}");
        }
    }
    #[test]
    fn composer_has_prompt_contrast_plain_input_and_visible_growth() {
        for theme in ["dark", "light", "mono"] {
            let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
            app.journal.theme = theme.into();
            app.composer
                .insert_paste("plain text\nmore text\nthird line");
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal
                .draw(|frame| app.render(frame, Duration::ZERO))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let mut found_prompt = false;
            let mut found_input = false;
            for y in 0..24 {
                let row: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
                if row.contains('›') && row.contains("plain text") {
                    found_prompt = true;
                    let start = row.find("plain text").unwrap() as u16;
                    found_input = true;
                    for x in start..start + 10 {
                        assert!(!buffer[(x, y)].modifier.contains(Modifier::UNDERLINED));
                    }
                    if theme != "mono" {
                        assert_ne!(buffer[(start, y)].bg, Color::Reset);
                    }
                }
            }
            assert!(
                found_prompt && found_input,
                "{theme}: composer prompt/input missing"
            );
            assert!(
                buffer
                    .content()
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>()
                    .contains("third line"),
                "{theme}: multiline draft missing"
            );
        }
    }
    #[test]
    fn delivery_evidence_is_retained_but_status_renders_once() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.scroll_back = 1;
        app.journal
            .submit("Clarify the alternatives minimum".into());
        assert!(app
            .journal
            .messages
            .iter()
            .any(|m| m.text.starts_with("Sending…")));
        assert_eq!(
            app.journal
                .messages
                .iter()
                .filter(|m| App::visible_message(m))
                .count(),
            1
        );
        app.advance();
        assert!(app
            .journal
            .messages
            .iter()
            .any(|m| m.text.starts_with("Request saved ·")));
        assert_eq!(
            app.journal
                .messages
                .iter()
                .filter(|m| App::visible_message(m))
                .count(),
            1
        );
        assert!(app.selected_status().contains("Request saved"));
        assert_eq!(
            app.journal.unread, 1,
            "only the visible user message is unread"
        );
    }
    #[test]
    fn action_focus_and_command_description_are_visible_at_eighty_columns() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.first = First::Suggested;
        app.focus = Focus::Activity;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(screen.contains("Create task proposal"));
        assert!(screen.contains("does not authorize"));
        assert!(buffer
            .content()
            .iter()
            .any(|c| c.modifier.contains(Modifier::REVERSED)));
        app.focus = Focus::Composer;
        app.input(key(KeyCode::Char('/')));
        app.input(key(KeyCode::Char('h')));
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(screen.contains("Help for this view"));
    }
    #[test]
    fn narrow_command_menu_shows_selected_unavailable_reason() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.composer = Composer::from_draft(Draft {
            text: "/include".into(),
            ..Default::default()
        });
        app.menu = true;
        let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(screen.contains("import creation"));
        assert!(screen.contains("no selected import"));
    }
    #[test]
    fn menu_enter_acts_tab_completes_and_paste_stays_literal() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.input(key(KeyCode::Char('/')));
        app.input(key(KeyCode::Char('h')));
        assert!(app.menu);
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Help, "Enter opens the selected command");
        assert!(!app.menu);
        assert_eq!(app.journal.first, First::Empty);
        app.input(key(KeyCode::Esc));
        app.input(key(KeyCode::Char('/')));
        app.input(key(KeyCode::Char('a')));
        app.input(key(KeyCode::Tab));
        assert!(!app.menu);
        assert!(app.composer.text().starts_with('/'));
        assert_eq!(app.journal.first, First::Empty);
        app.composer.clear();
        app.input(Event::Paste("/approve\npath".into()));
        assert!(!app.menu);
        assert!(app.composer.literal_paste);
    }
    #[test]
    fn slash_actions_use_the_same_panels_without_implicit_authorization() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        for (name, modal) in [("settings", Modal::Settings), ("tasks", Modal::Tasks)] {
            app.composer = Composer::from_draft(Draft { text: format!("/{name}"), ..Default::default() });
            app.menu = true;
            app.menu_index = 0;
            app.input(key(KeyCode::Enter));
            assert_eq!(app.modal, modal);
            app.input(key(KeyCode::Esc));
        }
        app.journal.first = First::Suggested;
        app.composer = Composer::from_draft(Draft { text: "/plan".into(), ..Default::default() });
        app.menu = true;
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Scope);
        assert_eq!(app.journal.first, First::Proposed);
        assert_eq!(app.journal.first_calls, 0);
        app.input(key(KeyCode::Esc));
        app.composer = Composer::from_draft(Draft { text: "/answer".into(), ..Default::default() });
        app.menu = true;
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::None);
        assert!(app.notice.contains("no pending question"));
        assert_eq!(app.composer.text(), "/answer");
    }
    #[test]
    fn user_rows_are_left_aligned_highlighted_and_coordinator_results_grouped() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.submit("first line\nsecond line".into());
        let message = app.journal.messages.iter().find(|m| m.speaker.starts_with("You")).unwrap();
        let text = app.message_text(message, 40);
        assert!(text.lines[0].spans[0].content.starts_with("> first line"));
        assert!(text.lines[1].spans[0].content.starts_with("  second line"));
        assert_eq!(text.lines[0].style.bg, Some(Color::Rgb(74,77,84)));
        assert_eq!(text.lines[1].style.bg, Some(Color::Rgb(74,77,84)));
        assert_eq!(text.lines[0].spans[0].content.len(), 40);
        let result = App::result_lines("one\ntwo", 40, Tone::for_theme("dark"));
        assert!(result[0].spans[0].content.starts_with("  └ "));
        assert!(result[1].spans[0].content.starts_with("    "));
        app.journal.add("Coordinator", "This response wraps across a narrow panel and retains its own following line.\nSecond paragraph");
        let coordinator = app.journal.messages.last().unwrap();
        let grouped = app.message_text(coordinator, 30);
        assert!(grouped.lines[0].spans[0].content.starts_with("• "));
        assert!(grouped.lines[0].spans[0].content.ends_with(" · Coordinator"));
        assert_eq!(grouped.lines[1].spans[0].content.as_ref(), "  └ ");
        assert!(grouped.lines[2].spans[0].content.starts_with("    "));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let buffer = terminal.backend().buffer();
        let screen = buffer.content().iter().map(|c| c.symbol()).collect::<String>();
        assert!(screen.contains("Wayward Wrangler"));
        let user_y = (0..24).find(|y| (0..80).map(|x| buffer[(x,*y)].symbol()).collect::<String>().contains("> first line")).unwrap();
        assert_eq!(buffer[(0,user_y)].bg, Color::Rgb(74,77,84));
        assert_eq!(buffer[(79,user_y)].bg, Color::Rgb(74,77,84));
        let input_row = (0..24).find(|y| (0..80).map(|x| buffer[(x,*y)].symbol()).collect::<String>().contains("›")).unwrap();
        assert_eq!(buffer[(0,input_row)].symbol(), "›");
        assert_eq!(buffer[(0,input_row)].bg, Color::Rgb(65,69,76));
    }
    #[test]
    fn conversation_uses_the_available_terminal_width_with_or_without_drawer() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.submit("A wide message".into());
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        assert_eq!(app.viewport_width, 160);
        let buffer = terminal.backend().buffer();
        let y = (0..48).find(|y| (0..160).map(|x| buffer[(x,*y)].symbol()).collect::<String>().contains("> A wide message")).unwrap();
        assert_eq!(buffer[(0,y)].bg, Color::Rgb(74,77,84));
        assert_eq!(buffer[(159,y)].bg, Color::Rgb(74,77,84));
        app.journal.drawer = true;
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        assert_eq!(app.viewport_width, 115);
        let buffer = terminal.backend().buffer();
        let y = (0..48).find(|y| (0..115).map(|x| buffer[(x,*y)].symbol()).collect::<String>().contains("> A wide message")).unwrap();
        assert_eq!(buffer[(114,y)].bg, Color::Rgb(74,77,84));
        assert_ne!(buffer[(115,y)].bg, Color::Rgb(74,77,84));
    }
    #[test]
    fn cursor_blink_is_presentation_only_and_input_restores_cursor() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let before: Vec<_> = terminal.backend().buffer().content().iter().map(|cell| cell.modifier).collect();
        app.cursor_changed = Instant::now() - Duration::from_millis(600);
        assert!(!app.tick());
        assert!(!app.cursor_visible);
        assert!(!app.persist_pending);
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let after: Vec<_> = terminal.backend().buffer().content().iter().map(|cell| cell.modifier).collect();
        assert_ne!(before, after, "blink must change a rendered cursor cell");
        app.input(key(KeyCode::Char('x')));
        assert!(app.cursor_visible);
        assert_eq!(app.composer.text(), "x");
    }
    #[test]
    fn startup_title_effect_is_once_and_disabled_in_reduced_modes() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(dir.path().to_path_buf()).unwrap();
        assert!(app.effects_running());
        app.effect = None;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        assert!(!app.effects_running(), "redraw cannot restart the title animation");
        for mode in ["reduced", "off"] {
            app.journal.effects = mode.into();
            app.save().unwrap();
            let reopened = App::new(dir.path().to_path_buf()).unwrap();
            assert!(!reopened.effects_running(), "{mode} must not animate on startup");
        }
    }
    #[test]
    fn drawer_and_settings_do_not_change_target_or_authorize() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.input(key(KeyCode::F(3)));
        assert_eq!(app.modal, Modal::Settings);
        app.input(key(KeyCode::Char('e')));
        assert_eq!(app.journal.effects, "reduced");
        app.input(key(KeyCode::Esc));
        app.input(key(KeyCode::F(6)));
        assert_eq!(app.modal, Modal::Tasks);
        assert_eq!(app.journal.target, Target::Coordinator);
        assert_eq!(app.journal.first, First::Empty);
    }
    #[test]
    fn coherent_scripted_session_uses_explicit_gates_and_survives_typing() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(dir.path().to_path_buf()).unwrap();
        app.input(Event::Paste(
            "Clarify remaining Decision Log choices; keep free editing".into(),
        ));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.first, First::PendingDelivery);
        app.advance();
        app.advance();
        assert_eq!(app.journal.first, First::Suggested);
        app.focus = Focus::Activity;
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Scope);
        assert_eq!(app.journal.first_calls, 0);
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.first, First::Authorized);
        app.input(Event::Paste(
            "I can keep drafting while the fake worker progresses".into(),
        ));
        let draft = app.composer.text();
        app.advance();
        app.advance();
        app.advance();
        assert_eq!(app.journal.first, First::Question);
        assert_eq!(app.composer.text(), draft);
        app.input(key(KeyCode::F(4)));
        app.input(key(KeyCode::Down));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.target, Target::NewTask);
        app.input(Event::Paste("Prepare a short manual walkthrough".into()));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.second, Second::Proposed);
        app.journal.selected_task = 1;
        app.focus = Focus::Activity;
        app.input(key(KeyCode::Enter));
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.second, Second::AuthorizedQueued);
        app.journal.selected_task = 0;
        app.focus = Focus::Activity;
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Question);
        app.input(key(KeyCode::Char('1')));
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.journal.first, First::AnswerUnknown);
        assert_eq!(app.journal.first_calls, 2);
        app.save().unwrap();
        let mut resumed = App::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(resumed.journal.answer.as_deref(), Some("Allow none"));
        resumed.advance();
        resumed.advance();
        assert_eq!(resumed.journal.first, First::Review);
        resumed.focus = Focus::Activity;
        resumed.input(key(KeyCode::Enter));
        resumed.input(key(KeyCode::Tab));
        resumed.input(key(KeyCode::Enter));
        assert_eq!(resumed.modal, Modal::Artifact);
        resumed.input(key(KeyCode::Enter));
        resumed.input(key(KeyCode::Tab));
        resumed.input(key(KeyCode::Enter));
        assert_eq!(resumed.journal.first, First::Approved);
        for _ in 0..4 {
            resumed.advance();
        }
        assert_eq!(resumed.journal.second, Second::Uncertain);
        assert_eq!(resumed.journal.second_calls, 1);
        assert_eq!(resumed.journal.first_calls, 3);
    }
    #[test]
    fn scope_and_review_actions_remain_visible_at_eighty_columns() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        app.modal = Modal::Scope;
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let display = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(display.contains("Authorize"));
        assert!(display.contains("Cancel"));
        app.modal = Modal::Review;
        app.journal.first = First::Review;
        app.journal.candidate_sha256 = Some("candidate-sha".into());
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let display = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(display.contains("Read candidate"));
        assert!(!display.contains("[Approve]"));
        assert!(!display.contains("[Reject]"));
        app.journal.reviewed_sha256 = app.journal.candidate_sha256.clone();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let display = terminal.backend().buffer().content().iter()
            .map(|c| c.symbol()).collect::<String>();
        assert!(display.contains("[Approve]"));
        assert!(display.contains("[Reject]"));
    }
    #[test]
    fn pasted_approval_command_is_a_literal_message() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.input(Event::Paste("/approve".into()));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::None);
        assert_eq!(app.journal.first_request, "/approve");
        assert_eq!(app.journal.first, First::PendingDelivery);
    }
    #[test]
    fn accent_effect_finishes_and_reduced_modes_schedule_none() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        app.start_effect(Color::Yellow, 100);
        assert!(app.effects_running());
        terminal
            .draw(|frame| app.render(frame, Duration::from_millis(120)))
            .unwrap();
        assert!(!app.effects_running());
        app.dirty = false;
        assert!(!app.tick());
        assert!(!app.dirty, "idle state must not schedule a redraw");
        for mode in ["reduced", "off"] {
            app.journal.effects = mode.into();
            app.start_effect(Color::Green, 100);
            assert!(!app.effects_running(), "{mode} mode scheduled an effect");
        }
    }
    #[test]
    fn ten_thousand_scripted_updates_keep_a_bounded_display_window() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        for n in 0..10_000 {
            app.journal.add("Waywr", format!("scripted status {n}"));
        }
        assert_eq!(app.journal.messages.len(), 200);
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        terminal
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        assert_eq!(app.journal.first_calls, 0);
        assert_eq!(app.journal.coordinator_calls, 0);
    }
    #[test]
    fn question_other_paste_and_history_insert_do_not_submit() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.submit("Original coordinator message".into());
        app.input(Event::Key(KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(app.modal, Modal::History);
        app.input(key(KeyCode::Enter));
        assert_eq!(app.composer.text(), "Original coordinator message");
        assert_eq!(app.journal.coordinator_calls, 1);
        app.journal.first = First::Question;
        app.modal = Modal::Question;
        app.question_choice = Some(2);
        app.input(Event::Paste("Other policy text".into()));
        assert_eq!(app.question_text, "Other policy text");
        assert_eq!(app.journal.answer, None);
    }
    #[test]
    fn themes_and_detach_work_with_overlay_open() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        for theme in ["dark", "light", "mono"] {
            app.journal.theme = theme.into();
            terminal
                .draw(|frame| app.render(frame, Duration::ZERO))
                .unwrap();
        }
        app.open(Modal::Settings);
        app.input(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.detached);
        assert_eq!(app.journal.first, First::Empty);
    }

    #[test]
    #[ignore = "explicit 30-second simulated input latency acceptance probe"]
    fn thirty_second_input_and_event_probe() {
        use std::time::Instant;
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        app.journal.submit("Clarify Decision Log choices".into());
        app.advance();
        app.advance();
        app.journal.propose_first();
        app.journal.authorize(0);
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        let begin = Instant::now();
        let mut max_latency = Duration::ZERO;
        let mut samples = 0u32;
        while begin.elapsed() < Duration::from_secs(30) {
            let op = Instant::now();
            app.input(key(KeyCode::Char('x')));
            if samples % 15 == 0 {
                app.journal
                    .add("Waywr", format!("SCRIPTED progress event {samples}"));
            }
            app.tick();
            terminal
                .draw(|frame| app.render(frame, Duration::ZERO))
                .unwrap();
            max_latency = max_latency.max(op.elapsed());
            samples += 1;
            std::thread::sleep(Duration::from_millis(10));
        }
        eprintln!(
            "scripted_input_samples={samples} max_input_plus_render_ms={:.2} display_items={}",
            max_latency.as_secs_f64() * 1000.0,
            app.journal.messages.len()
        );
        assert!(max_latency < Duration::from_millis(100));
        assert!(samples > 100);
    }

    #[test]
    fn scrolling_keeps_the_same_message_when_output_arrives_or_width_changes() {
        let mut app = App::new(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap();
        for n in 0..40 {
            app.journal.add("Waywr", format!("Scripted paragraph {n} with a long decision-log explanation that wraps at narrow widths."));
        }
        let mut narrow = Terminal::new(TestBackend::new(80, 24)).unwrap();
        narrow
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        app.input(key(KeyCode::PageUp));
        narrow
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        let anchor = app
            .journal
            .scroll_anchor_seq
            .expect("scroll should anchor a message");
        let top = app.viewport_top;
        app.journal.add(
            "Waywr",
            "A new event while the operator reads earlier messages",
        );
        narrow
            .draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        assert_eq!(app.journal.scroll_anchor_seq, Some(anchor));
        assert_eq!(app.viewport_top, top);
        assert_eq!(app.journal.unread, 1);
        let mut wide = Terminal::new(TestBackend::new(120, 36)).unwrap();
        wide.draw(|frame| app.render(frame, Duration::ZERO))
            .unwrap();
        assert_eq!(app.journal.scroll_anchor_seq, Some(anchor));
    }

    #[test]
    fn command_history_arrows_restore_drafts_without_scrolling_or_execution() {
        let mut app = scope_app();
        app.input(Event::Paste("Earlier prompt".into()));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::SubmitTask("Earlier prompt".into())));
        app.composer = Composer::from_draft(Draft { text: "/plan".into(), col: 5, ..Draft::default() });
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Scope);
        app.input(key(KeyCode::Esc));
        assert_eq!(app.input_history.len(), 2);
        for n in 0..30 { app.journal.add("Waywr", format!("Long chat message {n} that wraps to additional lines in a narrow terminal while input history remains separate.")); }
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let bottom = app.viewport_top;
        app.composer = Composer::from_draft(Draft { text: "Unsent\n続き".into(), row: 1, col: 2, multiline: true, literal_paste: true });
        let original = serde_json::to_value(app.composer.snapshot()).unwrap();
        app.input(key(KeyCode::Up));
        assert_eq!(app.composer.text(),"Unsent\n続き");
        let at_boundary=serde_json::to_value(app.composer.snapshot()).unwrap();
        app.input(key(KeyCode::Up));
        assert_eq!(app.composer.text(), "/plan");
        assert!(!app.composer.literal_paste);
        assert!(!app.menu);
        assert_eq!(app.modal, Modal::None);
        assert_eq!(app.viewport_top, bottom);
        app.input(key(KeyCode::Up));
        assert_eq!(app.composer.text(), "Earlier prompt");
        app.input(key(KeyCode::End));
        app.input(key(KeyCode::Char('!')));
        app.input(key(KeyCode::Up));
        assert_eq!(app.composer.text(), "Earlier prompt!", "stop at oldest and retain edits");
        app.input(key(KeyCode::Down));
        assert_eq!(app.composer.text(), "/plan");
        app.input(key(KeyCode::Down));
        assert_eq!(serde_json::to_value(app.composer.snapshot()).unwrap(), at_boundary);
        app.input(key(KeyCode::Down));
        assert_eq!(serde_json::to_value(app.composer.snapshot()).unwrap(), original);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::PageUp));
        assert_eq!(app.viewport_top, bottom - 8);
        assert_eq!(serde_json::to_value(app.composer.snapshot()).unwrap(), original);
        app.input(key(KeyCode::PageDown));
        assert_eq!(app.viewport_top, bottom);
        app.input(Event::Key(KeyEvent::new(KeyCode::PageUp,KeyModifiers::ALT)));
        assert_eq!(app.viewport_top,bottom-1);
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        assert_eq!(app.viewport_top,bottom-1);
        assert_eq!(serde_json::to_value(app.composer.snapshot()).unwrap(),original);
        app.input(Event::Key(KeyEvent::new(KeyCode::PageDown,KeyModifiers::ALT)));
        assert_eq!(app.viewport_top,bottom);
        app.input(Event::Key(KeyEvent::new(KeyCode::PageDown,KeyModifiers::ALT)));
        assert_eq!(app.viewport_top,bottom);
        app.input(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)));
        assert_eq!(app.composer.snapshot().row, 0);
        assert_eq!(app.composer.text(), "Unsent\n続き");
        assert_eq!(app.viewport_top, bottom);
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn command_history_picker_and_target_switch_preserve_unsent_input() {
        let mut app = continuous_app();
        app.input_history.remember(Draft { text: "/approve".into(), literal_paste: true, ..Draft::default() });
        app.composer.area.insert_str("Keep draft");
        app.input(Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)));
        assert_eq!(app.modal, Modal::History);
        assert!(app.modal_body().1.contains("/approve"));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let screen = terminal.backend().buffer().content().iter().map(|cell|cell.symbol()).collect::<String>();
        assert!(screen.contains("Command history"));
        assert!(screen.contains("[Recall entry]"));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::None);
        assert_eq!(app.composer.text(), "/approve");
        assert!(app.composer.literal_paste);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Down));
        assert_eq!(app.composer.text(), "Keep draft");
        app.input(key(KeyCode::Up));
        app.set_target(Target::NewTask);
        let new_subject=app.conversation_subject.clone();
        app.set_target(Target::Coordinator);
        assert_eq!(app.conversation_subject,new_subject);
        assert_eq!(app.composer.text(), "", "A new request has its own draft");
        app.discuss_selected_request();
        assert_eq!(app.composer.text(), "Keep draft");
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Up));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::SubmitTask("/approve".into())));
        assert_eq!(app.modal, Modal::None, "literal slash recall stays a message");
    }

    #[test]
    fn command_history_seeds_only_recorded_user_text_once() {
        let mut app = App::new_live("History project".into(), Draft::default());
        let snapshot = serde_json::json!({"tasks":[],"session":{"phase":"paused"},"recent_events":[],
            "conversation":{"messages":[{"message_id":"old-user","seq":1,"role":"user","text":"/plan"},
                {"message_id":"assistant","seq":2,"role":"assistant","text":"Do not recall me"}]}});
        app.apply_snapshot(&snapshot);
        app.apply_snapshot(&snapshot);
        assert_eq!(app.input_history.len(), 1);
        app.input(key(KeyCode::Up));
        assert_eq!(app.composer.text(), "/plan");
        assert!(app.composer.literal_paste, "past message records cannot establish command intent");
        assert!(app.take_intent().is_none());
        let mut empty = App::new_live("Another project".into(), Draft::default());
        empty.composer.area.insert_str("Unsent");
        empty.input(key(KeyCode::Up)); empty.input(key(KeyCode::Down));
        assert_eq!(empty.composer.text(), "Unsent");
        assert!(empty.input_history.is_empty());
    }

    #[test]
    fn installed_view_uses_accepted_editor_and_never_advances_scripted_scenes() {
        let mut app=App::new_live("Disposable project".into(),Draft::default());
        app.input(key(KeyCode::Char('b')));
        app.input(key(KeyCode::Char('u')));
        app.input(key(KeyCode::Char('i')));
        app.input(key(KeyCode::Char('l')));
        app.input(key(KeyCode::Char('d')));
        app.input(Event::Key(KeyEvent::new(KeyCode::Enter,KeyModifiers::SHIFT)));
        app.input(key(KeyCode::Char('x')));
        assert_eq!(app.composer.text(),"build\nx");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::SubmitTask("build\nx".into())));
        assert_eq!(app.journal.first,First::Empty);
        app.input(key(KeyCode::F(8)));
        assert_eq!(app.journal.first,First::Empty);
        app.input(Event::Paste("/exit".into()));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::SubmitTask("/exit".into())));
        for (width,height) in [(80,24),(160,48)] {
            let mut terminal=Terminal::new(TestBackend::new(width,height)).unwrap();
            terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
            let display=format!("{:?}",terminal.backend().buffer());
            assert!(display.contains("Disposable project"));
            assert!(display.contains("SIMULATED SERVICE"));
        }
    }

    #[test]
    fn installed_scope_requires_explicit_authorization_and_exit_detaches() {
        let mut app=App::new_live("A".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log","status":"proposed"}],
            "session":{"phase":"running"},"recent_events":[]}));
        app.command("plan");
        assert_eq!(app.take_intent(),Some(LiveIntent::Plan));
        app.apply_plan(serde_json::json!({"plan":{"task_id":"t-1","brief":"Specification"},"plan_digest":"digest"}));
        app.input(key(KeyCode::Enter));
        assert!(app.take_intent().is_none());
        app.command("plan");
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::Authorize));
        app.command("exit");
        assert!(app.detached);
        assert_eq!(app.take_intent(),Some(LiveIntent::Detach));
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn installed_next_action_opens_gate_without_submitting_or_deciding() {
        let mut app = App::new_live("project-a".into(), Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log","status":"proposed","version":1}],
            "session":{"phase":"running"},"recent_events":[]}));
        assert_eq!(app.action_label(), "Review scope");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::Plan));
        assert_eq!(app.modal, Modal::None);
        app.apply_plan(serde_json::json!({"plan":{"task_id":"t-1"},"plan_digest":"digest"}));
        assert_eq!(app.modal, Modal::Scope);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Esc));

        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log","status":"awaiting-input","version":2}],
            "session":{"phase":"running"},"recent_events":[]}));
        assert_eq!(app.action_label(), "Loading next action");
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":2},
            "pending":{"kind":"choice","request_id":"q-1","question":{"question":"Edit policy?","options":["free","append-only"]}}}));
        assert_eq!(app.action_label(), "Answer question");
        assert_eq!(
            app.modal,
            Modal::None,
            "background detail refresh must not steal focus"
        );
        let mut terminal=Terminal::new(TestBackend::new(80,24)).unwrap();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let display=format!("{:?}",terminal.backend().buffer());
        assert!(display.contains("Answer question"));
        assert!(display.contains("Enter with empty input"));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Question);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Esc));
        app.composer.area.insert_str("draft in progress");
        let draft = app.composer.snapshot();
        app.journal.scroll_back=2;
        app.focus = Focus::Activity;
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":2},
            "pending":{"kind":"choice","request_id":"q-1","question":{"question":"Edit policy?","options":["free","append-only"]}}}));
        assert_eq!(app.journal.scroll_back,2);
        assert_eq!(app.focus,Focus::Activity);
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Question);
        assert_eq!(app.composer.snapshot().text, draft.text);
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn installed_exact_candidate_refresh_and_explicit_approval() {
        let mut app = App::new_live("project-a".into(), Draft::default());
        let snapshot = serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log","status":"awaiting-input","version":3}],
            "session":{"phase":"running"},"recent_events":[]});
        let detail = serde_json::json!({"task":{"task_id":"t-1","version":3},
            "pending":{"kind":"final-review","request_id":"a-1","artifact_sha256":"sha-1"}});
        app.apply_snapshot(&snapshot);
        app.apply_detail(detail.clone());
        assert_eq!(app.action_label(), "Review candidate");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Review);
        assert_eq!(app.modal_action, 1, "read is the first visible review step");
        assert!(app.take_intent().is_none());
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let display = format!("{:?}", terminal.backend().buffer());
        assert!(display.contains("Read candidate"));
        assert!(!display.contains("[Approve]"));
        assert!(!display.contains("[Reject]"));
        app.input(key(KeyCode::Tab));
        assert_eq!(app.modal_action, 0, "unread review cycles only Back and Read");
        app.input(key(KeyCode::Tab));
        assert_eq!(app.modal_action, 1);
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::OpenArtifact));
        app.apply_artifact_page(
            &serde_json::json!({"task_id":"t-1","sha256":"sha-1","offset":0,
            "next_offset":5,"complete":true}),
            "hello",
        );
        app.input(key(KeyCode::Esc));
        assert_eq!(app.modal, Modal::Review);
        assert_eq!(
            app.modal_action, 2,
            "return offers a separate approval action"
        );
        app.apply_snapshot(&snapshot);
        app.apply_detail(detail.clone());
        assert_eq!(
            app.live.as_ref().unwrap().reviewed_sha.as_deref(),
            Some("sha-1")
        );
        assert_eq!(app.modal_action, 2);
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::Approve));
        assert!(
            app.take_intent().is_none(),
            "one confirmation emits one command intent"
        );
        let mut changed = detail.clone();
        changed["pending"]["request_id"] = "a-2".into();
        app.apply_detail(changed);
        assert!(app.live.as_ref().unwrap().reviewed_sha.is_none());
        assert!(app.live.as_ref().unwrap().artifact.is_none());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log","status":"approved","version":4}],
            "session":{"phase":"running"},"recent_events":[]}));
        assert_eq!(app.action_label(), "View result");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal, Modal::Inspect);
        assert_eq!(app.take_intent(), Some(LiveIntent::Inspect));
    }

    #[test]
    fn file_command_opens_only_selected_file_without_marking_bundle_reviewed(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Weather","status":"awaiting-input","version":1}],
            "session":{"phase":"running"},"recent_events":[]}));
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":1},
            "pending":{"kind":"final-review","request_id":"a-1","artifact_sha256":"manifest-1"}}));
        app.composer.area.insert_str("/open app.js");
        app.menu=true;
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::OpenFile("app.js".into())));
        app.apply_file_page(&serde_json::json!({"task_id":"t-1","artifact":"app.js","sha256":"file-1",
            "manifest_sha256":"manifest-1","offset":0,"complete":true}),"const data={};");
        assert!(!app.candidate_read());
        assert_eq!(app.live.as_ref().unwrap().preview_file.as_deref(),Some("app.js"));
    }

    #[test]
    fn installed_gate_changes_with_same_task_version_require_fresh_detail() {
        let mut app=App::new_live("project-a".into(),Draft::default());
        let row=|revision|serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log",
            "status":"awaiting-input","version":1,"state_revision":revision}],
            "session":{"phase":"running"},"recent_events":[]});
        app.apply_snapshot(&row(6));
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":1,"state_revision":6},
            "pending":{"kind":"choice","request_id":"q-1","question":{"options":["7 days"]}}}));
        assert_eq!(app.action_label(),"Answer question");
        app.apply_snapshot(&row(8));
        assert_eq!(app.action_label(),"Loading next action");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal,Modal::None,"stale question must not reopen");
        assert_eq!(app.take_intent(),Some(LiveIntent::Inspect));
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":1,"state_revision":8},
            "pending":{"kind":"final-review","request_id":"a-1","artifact_sha256":"sha-1"}}));
        assert_eq!(app.modal,Modal::Review);
        assert_eq!(app.action_label(),"Review candidate");
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn unavailable_answer_has_no_save_action_or_answer_intent() {
        let mut app = App::new_live("project-a".into(), Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log",
            "status":"awaiting-input","version":3,"state_revision":8}],
            "session":{"phase":"running"},"recent_events":[]}));
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":3,"state_revision":8},
            "pending":{"kind":"final-review","request_id":"a-1","artifact_sha256":"sha-1"}}));
        assert_eq!(app.enabled_reason("answer"), Some("selected task has no current choice question"));
        app.open(Modal::Question);
        app.modal_action = 1; // A question was open before the gate changed.
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let display = format!("{:?}", terminal.backend().buffer());
        assert!(display.contains("No question pending"));
        assert!(!display.contains("Save answer"));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal_action, 0);
        assert!(app.take_intent().is_none());
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":3,"state_revision":8},
            "pending":{"kind":"choice","request_id":"q-2","question":{"question":"Retention?",
                "options":["7 days","30 days"]}}}));
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let display = format!("{:?}", terminal.backend().buffer());
        assert!(display.contains("Save answer"));
        app.input(key(KeyCode::Down));
        assert_eq!(app.question_choice, Some(0), "question arrows still choose options");
        assert_eq!(app.modal_scroll, 0);
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(), Some(LiveIntent::Answer("7 days".into())));
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn schema_four_question_and_consultation_are_visible_actions_without_implicit_decision(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        let question=serde_json::json!({"question_id":"q-1","status":"open","prompt":"Which units?",
            "options":["Celsius only","Celsius/Fahrenheit toggle"]});
        app.apply_snapshot(&serde_json::json!({"tasks":[],"session":{"phase":"running"},
            "conversation":{"schema_version":1,"messages":[],"questions":[question],
                "actions":[],"budgets":{"conversation":{"charged":1,"limit":6},
                    "consultation":{"charged":0,"limit":1}}},"recent_events":[]}));
        assert_eq!(app.journal.target,Target::Coordinator);
        assert_eq!(app.action_label(),"Answer question");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal,Modal::Question);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Down));
        app.input(key(KeyCode::Tab));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::Answer("Celsius only".into())));
        assert!(app.take_intent().is_none());
        app.apply_snapshot(&serde_json::json!({"tasks":[],"session":{"phase":"running"},
            "conversation":{"schema_version":1,"messages":[{"message_id":"m-1","role":"user","text":"Consult"}],
                "questions":[],"actions":[{"action_id":"a-1","kind":"request_consultation",
                    "body":{"kind":"request_consultation","payload":{"question":"Check clarity"}}}],
                "budgets":{"conversation":{"charged":2,"limit":6},"consultation":{"charged":0,"limit":1}}},
            "recent_events":[]}));
        assert_eq!(app.action_label(),"Review consultation");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal,Modal::Consultation);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Tab));app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::Consult));
        assert!(app.take_intent().is_none());
    }
    #[test]
    fn live_mock_composer_distinguishes_provider_from_service_connection(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[],"session":{"phase":"running"},
            "conversation":{"schema_version":1,"messages":[],"questions":[],"actions":[],
                "binding":{"provider":"fake","model":"deterministic"}},"recent_events":[]}));
        let backend=TestBackend::new(100,36);
        let mut terminal=Terminal::new(backend).unwrap();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let display=format!("{:?}",terminal.backend().buffer());
        assert!(display.contains("Mock coordinator conversation"));
        assert!(!display.contains("not connected"));
        app.journal.connection="reconnecting".into();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let display=format!("{:?}",terminal.backend().buffer());
        assert!(display.contains("reconnecting; draft preserved"));
    }
    #[test]
    fn schema_four_mock_replacement_requires_settings_action(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[],"session":{"phase":"running"},
            "conversation":{"schema_version":1,"messages":[],"questions":[],"actions":[],
                "binding":{"generation":1,"provider":"fake","model":"deterministic","effort":"none"}},
            "recent_events":[]}));
        app.open(Modal::Settings);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Tab));app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::ReplaceCoordinator));
        assert!(app.take_intent().is_none());
    }
    #[test]
    fn schema_four_steering_opens_review_without_mutating_task(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Weather",
            "status":"proposed","version":1,"state_revision":1}],"session":{"phase":"running"},
            "conversation":{"schema_version":1,"messages":[],"questions":[],
                "actions":[{"action_id":"a-1","kind":"propose_steering",
                    "body":{"kind":"propose_steering","payload":{"task_id":"t-1","text":"Use local data"}}}]},
            "recent_events":[]}));
        assert_eq!(app.action_label(),"Review steering");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.modal,Modal::Steering);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Tab));app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::ApplySteering));
        assert!(app.take_intent().is_none());
    }

    #[test]
    fn content_panels_scroll_by_line_and_stop_at_wrapped_content_end() {
        let mut app = App::new_live("project-a".into(), Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"Decision log",
            "status":"blocked","version":1}],"session":{"phase":"running"},"recent_events":[]}));
        let long_evidence = "launch evidence and retained output\n".repeat(100);
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-1","version":1},
            "coordinator":{"evidence":long_evidence},"pending":null}));
        app.open(Modal::Inspect);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        assert!(app.modal_scroll_limit > 8);
        app.input(key(KeyCode::End));
        assert_eq!(app.modal_scroll,app.modal_scroll_limit,"End reaches the bottom of a content panel");
        app.input(key(KeyCode::Home));
        assert_eq!(app.modal_scroll,0);
        app.input(key(KeyCode::Down));
        assert_eq!(app.modal_scroll, 1);
        app.input(key(KeyCode::Up));
        assert_eq!(app.modal_scroll, 0);
        app.input(Event::Key(KeyEvent::new(KeyCode::PageDown,KeyModifiers::ALT)));assert_eq!(app.modal_scroll,1);
        app.input(Event::Key(KeyEvent::new(KeyCode::PageUp,KeyModifiers::ALT)));assert_eq!(app.modal_scroll,0);
        for _ in 0..app.modal_scroll_limit as usize + 10 {
            app.input(key(KeyCode::PageDown));
        }
        assert_eq!(app.modal_scroll, app.modal_scroll_limit);
        app.input(key(KeyCode::Down));
        assert_eq!(app.modal_scroll, app.modal_scroll_limit);
        app.input(key(KeyCode::Up));
        assert_eq!(app.modal_scroll, app.modal_scroll_limit - 1);
        let previous_limit = app.modal_scroll_limit;
        app.live.as_mut().unwrap().detail.as_mut().unwrap()["coordinator"] =
            serde_json::json!({"evidence":"short"});
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        assert!(app.modal_scroll_limit < previous_limit);
        assert_eq!(app.modal_scroll, app.modal_scroll_limit);
        app.input(key(KeyCode::PageDown));
        assert_eq!(app.modal_scroll, app.modal_scroll_limit);
    }

    #[test]
    fn installed_events_deduplicate_and_task_selection_survives_reordering() {
        let mut app=App::new_live("A".into(),Draft::default());
        let snapshot=serde_json::json!({"tasks":[{"task_id":"a","title":"A","status":"queued"},
            {"task_id":"b","title":"B","status":"proposed"}],"session":{"phase":"running"},
            "recent_events":[{"seq":1,"type":"task.queued","task_id":"a","body":{"text":"first"}}]});
        app.apply_snapshot(&snapshot);
        app.journal.selected_task=1;
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"b","title":"B","status":"proposed"},
            {"task_id":"a","title":"A","status":"queued"}],"session":{"phase":"running"},
            "recent_events":snapshot["recent_events"]}));
        assert_eq!(app.journal.selected_task,0);
        assert_eq!(app.journal.messages.len(),1);
        assert_eq!(app.journal.messages[0].speaker,"You");
    }

    #[test]
    fn blocked_coordinator_is_stopped_with_inspection_action() {
        let mut app=App::new_live("A".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"A","status":"blocked","run_id":null}],
            "session":{"phase":"running"},"recent_events":[{"seq":1,"type":"task.state_changed","task_id":"t-1",
            "body":{"status":"blocked","coordinator_execution":"uncertain","detail":"Runner disappeared without completion evidence"}}]}));
        let text=app.conversation_text(100).lines.iter().map(|line|line.spans.iter()
            .map(|span|span.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Stopped · Coordinator stopped"),"{text}");
        assert!(text.contains("Open /inspect"),"{text}");
        assert!(!text.contains("Done ·"),"{text}");
        assert!(!text.contains("Runner disappeared"),"{text}");
        assert!(app.tasks_text().contains("workflow not started"));
    }

    #[test]
    fn bootstrap_refusal_explains_no_proposal_without_claiming_completion() {
        let mut app=App::new_live("A".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"tasks":[{"task_id":"t-1","title":"A","status":"blocked","run_id":null}],
            "session":{"phase":"running"},"recent_events":[{"seq":1,"type":"task.state_changed","task_id":"t-1",
            "body":{"status":"blocked","coordinator_execution":"pre-send-refusal",
                "detail":"Bootstrap tab contains foreground work after 2 seconds; no worker launch was sent"}}]}));
        let text=app.conversation_text(100).lines.iter().map(|line|line.spans.iter()
            .map(|span|span.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Stopped · Coordinator stopped"),"{text}");
        assert!(text.contains("before launch"),"{text}");
        assert!(!text.contains("Done ·"),"{text}");
    }
    #[test]
    fn native_chat_readiness_stream_and_draft_are_honest(){
        let mut app=App::new_live("chat-project".into(),Draft::default());
        let mut snapshot=serde_json::json!({"tasks":[],"session":{"phase":"running"},"recent_events":[],
            "conversation":{"messages":[],"questions":[],"actions":[],"turns":[],"binding":{"provider":"claude","model":"claude-sonnet-5-5","generation":1},
              "budgets":{"conversation":{"charged":0,"limit":3}},"chat":{"status":"awaiting-authorization","reason":"Review settings","profile":{"mode":"native","model":"claude-sonnet-5-5","effort":"medium"}}}});
        app.apply_snapshot(&snapshot);app.input(key(KeyCode::Enter));assert_eq!(app.modal,Modal::Settings);assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Esc));app.input(Event::Paste("ordinary follow-up".into()));app.input(key(KeyCode::Enter));
        assert_eq!(app.composer.text(),"ordinary follow-up");assert!(app.take_intent().is_none());
        snapshot["conversation"]["chat"]["status"]=json!("available");app.apply_snapshot(&snapshot);app.input(key(KeyCode::Enter));
        assert_eq!(app.take_intent(),Some(LiveIntent::SubmitTask("ordinary follow-up".into())));assert!(app.take_intent().is_none());
        let refused=app.submission_draft.take().unwrap();assert_eq!(refused.text,"ordinary follow-up");assert!(refused.literal_paste);
        app.composer=Composer::from_draft(refused.clone());let restored=app.composer.snapshot();assert_eq!((restored.text,restored.row,restored.col,restored.multiline,restored.literal_paste),(refused.text,refused.row,refused.col,refused.multiline,refused.literal_paste));app.composer.clear();
        app.input(Event::Paste("unfinished draft".into()));snapshot["conversation"]["provisional"]=json!({"text":"Actual provider text, still provisional\u{001b}]52;forbidden\u{0007}"});
        app.apply_snapshot(&snapshot);assert_eq!(app.composer.text(),"unfinished draft");
        let backend=TestBackend::new(160,36);let mut terminal=Terminal::new(backend).unwrap();terminal.draw(|f|app.render(f,Duration::ZERO)).unwrap();
        let display=format!("{:?}",terminal.backend().buffer());assert!(display.contains("NATIVE CONVERSATION"));assert!(display.contains("Service connected"));assert!(display.contains("Actual provider text"));assert!(!display.contains("SIMULATED SERVICE"));
        assert!(app.conversation_text(120).lines.iter().flat_map(|line|&line.spans).all(|span|!span.content.contains('\u{001b}')&&!span.content.contains('\u{0007}')));
    }

    #[test]
    fn stopped_conversation_uses_authoritative_failure_label(){
        let mut app=App::new_live("project-a".into(),Draft::default());
        app.apply_event(&json!({"seq":1,"type":"conversation.turn_stopped","body":{"status":"uncertain","reason":"Runner exited without validated receipt"}}));
        let text=app.conversation_text(120).lines.iter().map(|line|line.spans.iter().map(|s|s.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Uncertain"));assert!(!text.contains("Done"));
    }

    #[test]
    fn inspect_uses_authoritative_chat_budget_before_any_proposal(){
        for limit in [3,6]{for width in [80,120]{
            let mut app=scope_app();let live=app.live.as_mut().unwrap();live.tasks.clear();live.detail=None;
            let c=live.conversation.as_mut().unwrap();c["scope_enabled"]=json!(limit==6);
            c["chat"]["profile"]["call_limit"]=json!(limit);c["chat"]["status"]=json!("blocked");
            c["budgets"]["conversation"]=json!({"charged":1,"limit":limit});
            app.open(Modal::Inspect);
            let expected=format!("Conversation allowance: 1/{limit} held or charged");
            assert!(app.live_modal_body().1.contains(&expected));
            assert!(app.live_modal_body().1.contains("a validated completion also saves runtime-result.json"));
            let mut terminal=Terminal::new(TestBackend::new(width,36)).unwrap();
            terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
            assert!(format!("{:?}",terminal.backend().buffer()).contains(&expected));
            assert!(app.take_intent().is_none());
        }}
    }
    pub(super) fn scope_app()->App{
        let mut app=App::new_live("scope-project".into(),Draft::default());
        app.apply_snapshot(&serde_json::json!({"state_revision":1,"tasks":[{"task_id":"t-scope","title":"Weather scope","version":1,"state_revision":1,"status":"proposed","run_id":null}],"session":{"phase":"running"},"recent_events":[],"conversation":{"scope_enabled":true,"chat":{"status":"available","profile":{"profile":"claude-proposal-v1","mode":"mock-cli","call_limit":6},"authorized":true},"messages":[],"questions":[],"actions":[],"turns":[],"binding":{"provider":"claude","model":"claude-sonnet-5-5","generation":1},"budgets":{"conversation":{"charged":3,"limit":6},"task":{"limit":0},"consultation":{"limit":0}}}}));
        app.apply_detail(serde_json::json!({"task":{"task_id":"t-scope","version":1,"state_revision":1},"scope":{"base":{"task_id":"t-scope","revision":1,"sha256":"sha-1"},"decision_revision":0,"authorization_command_id":null,"authorized":false,"needs_revision":false,"pending_turns":0,"questions":[],"record":{"proposal":{"title":"Weather scope","objective":"Fixed sample data","deliverables":["Page"],"in_scope":["Sample data"],"out_of_scope":["Network"],"constraints":[],"acceptance_criteria":["Sample data displays"],"unresolved_decisions":["Color"]}},"execution_available":false}}));app
    }
    #[test]fn scope_visible_action_opens_without_authority_and_keeps_normal_input(){
        let mut app=scope_app();assert_eq!(app.action_label(),"Review scope");app.input(key(KeyCode::Enter));assert_eq!(app.modal,Modal::Scope);assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Esc));app.input(Event::Paste("/approve is literal\nLooks good".into()));app.input(key(KeyCode::Enter));assert_eq!(app.take_intent(),Some(LiveIntent::SubmitTask("/approve is literal\nLooks good".into())));assert!(app.take_intent().is_none());
        app.command("plan");assert_eq!(app.modal,Modal::Scope);assert!(app.take_intent().is_none());
    }
    #[test]fn scope_pending_turn_refresh_enables_only_explicit_authorization(){
        let mut app=scope_app();app.input(key(KeyCode::Enter));
        let mut detail=app.live.as_ref().unwrap().detail.clone().unwrap();
        detail["scope"]["pending_turns"]=serde_json::json!(1);app.apply_detail(detail.clone());
        for _ in 0..3{app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());}
        let ack=app.scope_ack.clone();assert_eq!(app.scope_step,3);
        app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());
        detail["scope"]["pending_turns"]=serde_json::json!(0);app.apply_detail(detail);
        assert_eq!(app.scope_ack,ack);assert_eq!(app.modal,Modal::Scope);
        assert!(app.take_intent().is_none(),"refresh cannot authorize");
        app.input(key(KeyCode::Enter));assert!(matches!(app.take_intent(),Some(LiveIntent::AuthorizeScope(_))));
    }
    #[test]fn scope_deferred_detail_opens_requested_scope_without_background_focus_change(){
        let mut app=scope_app();let detail=app.live.as_mut().unwrap().detail.take().unwrap();
        app.input(key(KeyCode::Enter));assert_eq!(app.take_intent(),Some(LiveIntent::Inspect));
        assert_eq!(app.modal,Modal::None);app.apply_detail(detail.clone());assert_eq!(app.modal,Modal::Scope);
        app.input(key(KeyCode::Esc));app.composer.area.insert_str("unsent draft");
        app.apply_detail(detail);assert_eq!(app.modal,Modal::None);assert_eq!(app.focus,Focus::Composer);
        assert_eq!(app.composer.text(),"unsent draft");assert!(app.take_intent().is_none());
    }
    #[test]fn scope_ack_refresh_and_authorization_are_exact_explicit_actions(){
        let mut app=scope_app();app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());
        assert!(!app.scope_actions().iter().any(|(a,_)|*a==ScopeAction::Authorize));
        for _ in 0..3{app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());}
        let ack=app.scope_ack.clone();let detail=app.live.as_ref().unwrap().detail.clone().unwrap();app.apply_detail(detail);assert_eq!(app.scope_ack,ack);assert_eq!(app.scope_step,3);assert_eq!(app.modal,Modal::Scope);
        app.input(key(KeyCode::Enter));let Some(LiveIntent::AuthorizeScope(payload))=app.take_intent()else{panic!("explicit scope action missing")};assert_eq!(payload["base"]["sha256"],"sha-1");assert!(app.take_intent().is_none());
        let mut changed=app.live.as_ref().unwrap().detail.clone().unwrap();changed["scope"]["base"]["sha256"]=serde_json::json!("sha-2");app.apply_detail(changed);assert!(app.scope_ack.is_none());assert_eq!(app.scope_step,0);
    }
    #[test]fn scope_editor_preserves_paste_draft_and_detects_refresh_conflict(){
        let mut app=scope_app();app.composer.area.insert_str("main draft");app.scope_start_edit();app.input(Event::Key(KeyEvent::new(KeyCode::Char('a'),KeyModifiers::CONTROL)));app.input(Event::Paste("/approve\nNew objective".into()));app.input(key(KeyCode::Enter));let value=app.scope_edit_snapshot();assert!(value["payload"]["proposal"]["objective"].as_str().unwrap().starts_with("/approve\n"));assert!(app.take_intent().is_none());
        let mut restored=scope_app();restored.restore_scope_edit(value);assert!(restored.scope_editor.text().contains("New objective"));assert_eq!(restored.modal,Modal::None);
        let mut changed=app.live.as_ref().unwrap().detail.clone().unwrap();changed["scope"]["base"]["sha256"]=serde_json::json!("new");app.apply_detail(changed);assert_eq!(app.composer.text(),"main draft");assert_eq!(app.modal,Modal::ScopeEditor);assert!(app.scope_editor.text().contains("New objective"));assert!(app.live_modal_body().1.contains("retained edit"));
        app.input(key(KeyCode::Tab));app.input(key(KeyCode::Enter));let Some(LiveIntent::SaveScope(payload))=app.take_intent()else{panic!("save missing")};assert_eq!(payload["base"]["sha256"],"sha-1","must retain original base for backend refusal");assert_eq!(app.composer.text(),"main draft");
    }
    #[test]fn scope_answer_saved_shows_continue_and_never_save_again(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["questions"]=serde_json::json!([{"question_id":"q","status":"answered","prompt":"Unit?","answer":"Toggle","scope":{"continuation_available":true,"target_task_id":null}}]);
        assert_eq!(app.action_label(),"Continue with coordinator · uses 1 call");app.input(key(KeyCode::Enter));let mut t=Terminal::new(TestBackend::new(80,24)).unwrap();t.draw(|f|app.render(f,Duration::ZERO)).unwrap();let display=format!("{:?}",t.backend().buffer());assert!(!display.contains("[Save answer]"));app.input(key(KeyCode::Tab));app.input(key(KeyCode::Enter));assert_eq!(app.take_intent(),Some(LiveIntent::ContinueAnswer));
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["chat"]["status"]=serde_json::json!("exhausted");app.open(Modal::Question);app.modal_action=1;app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());assert!(app.notice.contains("exhausted"));
    }
    #[test]fn scope_authorized_result_and_narrow_panels_do_not_offer_another_authorization(){
        for (w,h) in [(80,24),(120,36),(160,48)]{let mut app=scope_app();app.open(Modal::Scope);let mut t=Terminal::new(TestBackend::new(w,h)).unwrap();
            for step in 0..4{t.draw(|f|app.render(f,Duration::ZERO)).unwrap();let display=format!("{:?}",t.backend().buffer());assert!(display.contains(&format!("Step {} of 4",step+1)));assert!(display.contains("Edit scope"));assert_eq!(display.contains("[Authorize scope]"),step==3);for _ in 0..100{app.input(key(KeyCode::PageDown));}assert!(app.modal_scroll<=app.modal_scroll_limit);if step<3{app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());assert_eq!(app.modal_scroll,0);}}
            let live=app.live.as_mut().unwrap();live.tasks[0]["status"]=serde_json::json!("scope-authorized");live.detail.as_mut().unwrap()["scope"]["authorized"]=serde_json::json!(true);t.draw(|f|app.render(f,Duration::ZERO)).unwrap();let display=format!("{:?}",t.backend().buffer());assert!(!display.contains("[Authorize scope]"));assert_eq!(app.action_label(),"View authorized scope");assert!(app.take_intent().is_none());
        }
    }

    #[test]fn scope_steps_show_only_current_content_and_keep_edit_back_and_drafts(){
        let mut app=scope_app();app.composer.area.insert_str("unsent multiline\ndraft");app.open(Modal::Scope);
        assert!(app.scope_body().1.contains("**Objective**"));assert!(!app.scope_body().1.contains("**Acceptance criteria**"));
        app.input(key(KeyCode::Enter));assert!(app.scope_body().1.contains("**Acceptance criteria**"));assert!(!app.scope_body().1.contains("**Objective**"));
        app.modal_action=1;app.input(key(KeyCode::Enter));assert_eq!(app.scope_step,0);
        app.input(key(KeyCode::Enter));app.input(key(KeyCode::Enter));assert!(app.scope_body().1.contains("**Unresolved decisions**"));
        assert!(!app.scope_body().1.contains("**Deliverables**"));
        let edit=app.scope_actions().iter().position(|(a,_)|*a==ScopeAction::Edit).unwrap();app.modal_action=edit;app.input(key(KeyCode::Enter));assert_eq!(app.modal,Modal::ScopeEditor);
        app.input(key(KeyCode::Esc));assert_eq!(app.modal,Modal::Scope);assert_eq!(app.scope_step,2);assert_eq!(app.composer.text(),"unsent multiline\ndraft");
        app.input(key(KeyCode::Enter));assert_eq!(app.scope_step,3);assert!(app.scope_body().1.contains("**Final decision**"));assert!(app.take_intent().is_none());
        app.modal_action=app.scope_actions().iter().position(|(a,_)|*a==ScopeAction::Close).unwrap();app.input(key(KeyCode::Enter));assert_eq!(app.modal,Modal::None);assert!(app.take_intent().is_none());
    }
    #[test]fn scope_repeat_keys_never_acknowledge_or_authorize_and_refresh_keeps_position(){
        let mut app=scope_app();app.open(Modal::Scope);
        let mut repeated=KeyEvent::new(KeyCode::Enter,KeyModifiers::NONE);repeated.kind=KeyEventKind::Repeat;
        for _ in 0..10{app.input(Event::Key(repeated));}assert_eq!(app.scope_step,0);assert!(app.scope_ack.is_none());assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Enter));app.modal_scroll=2;app.modal_action=2;
        let detail=app.live.as_ref().unwrap().detail.clone().unwrap();app.apply_detail(detail.clone());assert_eq!(app.scope_step,1);assert_eq!(app.modal_action,2);assert_eq!(app.modal_scroll,2);
        let mut changed=detail;changed["scope"]["decision_revision"]=serde_json::json!(1);app.apply_detail(changed);assert_eq!(app.scope_step,0);assert_eq!(app.modal_action,0);assert_eq!(app.modal_scroll,0);assert!(app.take_intent().is_none());
    }
    #[test]fn scope_without_unresolved_choices_still_needs_separate_final_authorization(){
        let mut app=scope_app();app.live.as_mut().unwrap().detail.as_mut().unwrap()["scope"]["record"]["proposal"]["unresolved_decisions"]=json!([]);app.open(Modal::Scope);
        app.input(key(KeyCode::Enter));app.input(key(KeyCode::Enter));assert_eq!(app.scope_actions()[0].1,"Continue to authorization");app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Enter));assert!(matches!(app.take_intent(),Some(LiveIntent::AuthorizeScope(_))));
    }


    pub(super) fn continuous_app()->App{
        let mut a=scope_app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["continuous"]=json!({"subjects":[{"subject_id":"discussion","task_id":"t-scope"}],"pause":{"generation":2,"outcome_sha256":"failure-proof","recoverable":true},"queue":[]});a
    }
    #[test]
    fn modal_action_arrows_navigate_without_authorizing_or_losing_drafts() {
        for (modal, count) in [(Modal::Scope,4),(Modal::Review,2),(Modal::Target,2),
            (Modal::Detach,2),(Modal::Pause,2),(Modal::Cancel,2),(Modal::UnknownCommand,2),(Modal::Settings,2)] {
            let mut app=continuous_app();
            app.input(Event::Paste("Unsent draft".into()));
            app.open(modal);
            app.input(key(KeyCode::Right));assert_eq!(app.modal_action,1,"{modal:?}");
            app.input(key(KeyCode::Left));assert_eq!(app.modal_action,0,"{modal:?}");
            app.input(key(KeyCode::Left));assert_eq!(app.modal_action,count-1,"{modal:?}");
            assert_eq!(app.modal,modal);assert_eq!(app.composer.text(),"Unsent draft");
            assert!(app.take_intent().is_none(),"Navigation must not execute {modal:?}");
            assert_eq!(app.current_scope().unwrap()["authorized"],false);
        }
    }
    #[test]
    fn resume_distinguishes_session_pause_workflow_recovery_and_questions() {
        let mut app = continuous_app();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"] = json!({"workflows":[{
            "work_id":"t-scope","status":"attention","revision":17,"pending":null,"question":null}]});
        app.command("resume");
        let Some(LiveIntent::Dialogue(command)) = app.take_intent() else { panic!("Recovery unavailable"); };
        assert_eq!(command["action"], "development.resume"); assert_eq!(command["payload"]["revision"], 17);
        assert_eq!(command["payload"]["work_id"], "t-scope"); assert_eq!(app.modal, Modal::None);
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["workflows"][0]["question"] = json!({"prompt":"Choose a format"});
        app.command("resume"); assert!(app.notice.contains("/reply")); assert!(app.take_intent().is_none());
        app.live.as_mut().unwrap().phase = "paused".into();
        app.command("resume");
        let Some(LiveIntent::Dialogue(command)) = app.take_intent() else { panic!(); };
        assert_eq!(command["action"], "resume_session");
        app.live.as_mut().unwrap().phase = "running".into();
        let work=&mut app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["workflows"][0];
        work["question"] = Value::Null; work["status"] = json!("running");
        app.command("resume"); assert!(app.notice.contains("already running")); assert!(app.take_intent().is_none());
    }
    #[test]
    fn idle_ticks_remain_responsive_with_a_full_catalog_and_retained_work(){
        let mut app=continuous_app();app.modal=Modal::None;
        let models=(0..1600).map(|n|json!({"provider":"fixture","model":format!("model-{n}"),
            "name":"Model catalog metadata","context_window":128000,"thinking_levels":["low","medium","high"],
            "capabilities":["conversation","implementation","review"]})).collect::<Vec<_>>();
        let units=(0..46).map(|n|json!({"unit_id":format!("u-{n}"),"work_id":"old-request",
            "status":"accepted","retained_summary":"Retained display metadata. ".repeat(1000)})).collect::<Vec<_>>();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({
            "workflows":[{"work_id":"t-scope","status":"attention","pending":null}],
            "units":units,"providers":{"discovery":{"models":models}}});
        let started=Instant::now();
        for _ in 0..120{app.tick();}
        assert!(started.elapsed()<Duration::from_millis(500),"An idle tick must not copy the project catalog and retained history on every iteration: {:?}",started.elapsed());
    }
    #[test]
    fn escape_and_control_c_clear_the_composer_without_cancelling_work(){
        for code in [KeyEvent::new(KeyCode::Esc,KeyModifiers::NONE),KeyEvent::new(KeyCode::Char('c'),KeyModifiers::CONTROL)]{
            let mut app=continuous_app();app.modal=Modal::None;app.focus=Focus::Composer;
            app.input(Event::Paste("Unsent multiline\ndraft".into()));
            app.input(Event::Key(code));
            assert_eq!(app.composer.text(),"","Both clear controls must empty the main input");
            assert_eq!(app.modal,Modal::None);assert!(app.take_intent().is_none());
            app.input_history.remember(Draft{text:"Older message".into(),..Default::default()});
            app.input(Event::Paste("Original unsent draft".into()));
            app.input(Event::Key(KeyEvent::new(KeyCode::Up,KeyModifiers::NONE)));
            assert!(app.input_history.is_recalling());
            app.input(Event::Key(code));
            assert_eq!(app.input_history.draft_to_save(app.composer.snapshot()).text,"");
        }
    }
    #[test]
    fn coordinator_usage_is_visible_and_scoped_without_inventing_account_quota() {
        let mut app = continuous_app();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["turns"] = json!([{
            "turn_id":"call-one","subject_id":"discussion","status":"completed",
            "runtime":{"provider":"custom-protocol","model":"model-outside-core","thinking":"high",
                "activity":{"label":"Submitting result"},
                "usage":{"input":1234,"output":56,"cacheRead":200,"cacheWrite":0,
                    "cost":0.0123,"context_tokens":1490,"context_window":4096}}
        }]);
        let text = app.conversation_text(100).lines.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("custom-protocol/model-outside-core"), "Coordinator usage must name the actual call's model");
        assert!(text.contains("1234") && text.contains("56") && text.contains("$0.0123"));
        assert!(text.contains("2606") && text.contains("estimate"));
        assert!(text.contains("completed"));
        assert!(app.take_intent().is_none());
        for (width,height) in [(80,24),(120,32)] {
            let backend=TestBackend::new(width,height);let mut terminal=Terminal::new(backend).unwrap();
            terminal.draw(|f|app.render(f,Duration::ZERO)).unwrap();
            let buffer=terminal.backend().buffer();
            let display=(0..height).map(|y|(0..width).map(|x|buffer[(x,y)].symbol()).collect::<String>()).collect::<Vec<_>>().join("\n");
            assert!(display.contains("1234") && display.contains("$0.0123"),"Usage must be visible in the normal terminal viewport");
            if let Ok(dir)=std::env::var("WAYWR_UI_SNAPSHOT_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(std::path::Path::new(&dir).join(format!("coordinator-usage-{width}x{height}.txt")),display).unwrap();
            }
        }
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"] = json!({"version":1});
        app.conversation_subject="another-request".into();
        assert!(app.coordinator_activity_lines().is_empty(),"Another subject's usage must not appear in this conversation");
        app.conversation_subject="discussion".into();
        app.journal.connection="reconnecting".into();
        assert!(app.coordinator_activity_lines()[0].to_string().contains("Last observed"));
        app.journal.connection="connected".into();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["turns"][0]["runtime"]["usage"] = json!({});
        let text = app.conversation_text(100).lines.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("unknown") && !text.contains("$0.0000"));
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["turns"].as_array_mut().unwrap().insert(0,json!({
            "turn_id":"call-two","subject_id":"discussion","status":"reserved","runtime":null}));
        let text=app.coordinator_activity_lines().iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("not reported yet") && !text.contains("1234"));
        assert_eq!(app.coordinator_activity_request().unwrap()["turn_id"],"call-two");
        let response=json!({"turn_id":"call-two","status":"reserved","runtime":{
            "observed_at":waywr_backend::execution::now(),"provider":"custom-protocol","model":"model-two",
            "activity":{"label":"Requesting model response"},"usage":{"input":42,"output":5}}});
        app.apply_coordinator_activity(&response);
        let text=app.coordinator_activity_lines().iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("model-two") && text.contains("42") && text.contains("Requesting model response"));
        let mut older=response.clone();older["runtime"]["observed_at"]=json!(1.);older["runtime"]["model"]=json!("stale-model");
        app.apply_coordinator_activity(&older);
        assert!(!app.coordinator_activity_lines()[0].to_string().contains("stale-model"));
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["turns"][0]["runtime"]["observed_at"]=json!(1.);
        assert!(app.coordinator_activity_lines()[0].to_string().contains("refresh unavailable"));
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["turns"][0]["status"]=json!("completed");
        app.apply_coordinator_activity(&response);
        assert_eq!(app.live.as_ref().unwrap().conversation.as_ref().unwrap()["turns"][0]["status"],"completed");
        assert!(app.coordinator_activity_request().is_none());
    }
    #[test]
    fn activity_timestamps_use_local_today_or_date_without_inventing_missing_time() {
        let now = 1792157400.;
        assert_eq!(App::activity_time(Some(now), now).len(), 5);
        assert_eq!(App::activity_time(Some(now - 172800.), now).len(), 14);
        assert_eq!(App::activity_time(None, now), "time unavailable");
        let mut app = continuous_app();
        app.apply_event(&json!({"seq":100,"at":now,"type":"session.state_changed","body":{"phase":"running"}}));
        let message = app.journal.messages.last().unwrap();
        assert_eq!(message.at, Some(now));
        let text = app.message_text(message, 120).lines.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains(&App::activity_time(Some(now), waywr_backend::execution::now())));
        assert!(text.contains("Session"));
    }
    #[test]
    fn live_indicator_times_only_active_work_and_respects_motion_settings() {
        let mut app = continuous_app();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"] = json!({"workflows":[{
            "work_id":"t-scope","status":"running","pending":{"kind":"coordinate","started_at":9542.}}],"units":[]});
        let line = app.active_work_line(10000.).unwrap();
        assert!(line.to_string().contains("Coordinator working (7m 38s)"));
        assert_ne!(line.spans, app.active_work_line(10000.25).unwrap().spans);
        app.journal.effects = "reduced".into();
        assert_eq!(app.active_work_line(10000.).unwrap().spans, app.active_work_line(10000.25).unwrap().spans);
        app.journal.effects = "off".into();
        assert_eq!(app.active_work_line(10000.).unwrap().spans, app.active_work_line(10000.25).unwrap().spans);
        app.focus = Focus::Activity; app.effect = None; app.dirty = false;
        app.activity_changed = Instant::now() - Duration::from_secs(2); app.tick(); assert!(app.dirty);
        let work=&mut app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["workflows"][0];
        work["status"] = json!("attention"); work["pending"] = Value::Null;
        assert!(app.active_work_line(10000.).is_none()); app.dirty = false; app.tick(); assert!(!app.dirty);
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["units"] = json!([{"work_id":"other","status":"running","started_at":9000.}]);
        assert!(app.active_work_line(10000.).is_none());
        app.live.as_mut().unwrap().phase = "paused".into(); assert!(app.active_work_line(10000.).is_none());
    }
    fn worker_activity_app() -> App {
        let mut app = continuous_app();
        app.journal.connection = "connected".into();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"] = json!({
            "workflows":[{"work_id":"t-scope","status":"running","pending":null}],"units":[
            {"work_id":"t-scope","title":"Python storage","unit_id":"storage","status":"running","operation_id":"worker-storage"},
            {"work_id":"t-scope","unit_id":"cli","status":"running","operation_id":"worker-cli"},
            {"work_id":"other","title":"Foreign worker","status":"running"},
            {"work_id":"t-scope","title":"Finished worker","status":"integrated"}]});
        app
    }
    #[test]
    fn worker_activity_rows_follow_live_workers_without_disturbing_output_or_drafts() {
        let mut app = worker_activity_app();
        app.composer = Composer::from_draft(Draft { text:"Unsent steering".into(), ..Draft::default() });
        app.apply_operation_list(&json!({"task_id":"t-scope","operations":[
            {"stage":"worker-storage","attempt_id":"a"},{"stage":"worker-cli","attempt_id":"b"}]}));
        let live=app.live.as_mut().unwrap();
        live.worker_output="retained output".into();live.worker_cursor=json!({"offset":44});live.worker_index=1;
        app.apply_operation_list(&json!({"task_id":"t-scope","active_only":true,"operations":[
            {"stage":"worker-storage","attempt_id":"a","activity":{"label":"Running: python3 -m unittest"}},
            {"stage":"worker-cli","attempt_id":"b","activity":{"label":"Editing files"}}]}));
        let text=app.worker_activity_lines().iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("└ • Python storage"));assert!(text.contains("• cli"));
        assert!(text.contains("python3 -m unittest"));assert!(text.contains("Editing files"));
        assert!(!text.contains("Foreign"));assert!(!text.contains("Finished"));assert!(!text.contains("retained output"));
        let live=app.live.as_ref().unwrap();assert_eq!(live.operations.len(),2);
        assert_eq!(live.worker_index,1);assert_eq!(live.worker_output,"retained output");
        assert_eq!(live.worker_cursor["offset"],44);assert_eq!(app.composer.text(),"Unsent steering");
        app.apply_operation_list(&json!({"task_id":"other","active_only":true,"operations":[]}));
        assert_eq!(app.live.as_ref().unwrap().worker_activities.len(),2);
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["units"][0]["attempt_id"]=json!("new-attempt");
        assert!(!app.worker_activity_lines().iter().any(|line|line.to_string().contains("python3 -m unittest")));
        app.live.as_mut().unwrap().activity_fetched=Some(Instant::now()-Duration::from_secs(6));
        assert!(!app.worker_activity_lines().iter().any(|line|line.to_string().contains("python3 -m unittest")));
        app.live.as_mut().unwrap().phase="paused".into();assert!(app.worker_activity_lines().is_empty());
    }
    #[test]
    fn recovery_events_are_readable_deduplicated_and_do_not_capture_ordinary_chat(){
        let mut app=continuous_app();
        let recovery=json!({"cause":"uncertain_execution","phase":"blocked","detail":"Checking the exact saved execution records","next_action":"Validate the original receipt","wake_condition":"Positive settlement","decision_needed":false,"can_continue":"Conversation and independent tasks"});
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[{"work_id":"t-scope","status":"running","recovery":recovery}],"units":[]});
        let event=json!({"seq":100,"type":"development.recovery","body":{"recovery":recovery}});
        app.apply_event(&event);let count=app.journal.messages.len();app.apply_event(&event);assert_eq!(app.journal.messages.len(),count);
        let text=&app.journal.messages.last().unwrap().text;
        assert!(text.contains("Positive settlement"));assert!(text.contains("No new approval"));assert!(text.contains("independent tasks"));
        app.input(Event::Paste("What is waiting?\nPlease explain the retained evidence.".into()));
        app.input(key(KeyCode::Enter));assert_eq!(app.take_intent(),Some(LiveIntent::SubmitTask("What is waiting?\nPlease explain the retained evidence.".into())));assert!(app.take_intent().is_none());
        assert_eq!(app.development_workflow().unwrap()["recovery"]["phase"],"blocked","Chat cannot clear execution uncertainty");
    }
    #[test]
    fn recovery_worker_status_preserves_independent_activity_and_saved_draft(){
        let mut app=worker_activity_app();app.composer.area.insert_str("Unsent steering");
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["units"][0]["recovery"]=json!({"phase":"reconciling","detail":"Validating the same receipt"});
        app.apply_operation_list(&json!({"task_id":"t-scope","active_only":true,"operations":[{"stage":"worker-cli","activity":{"label":"Running: python3 -m unittest"}}]}));
        let text=app.worker_activity_lines().iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Recovery reconciling"));assert!(text.contains("Validating the same receipt"));assert!(text.contains("python3 -m unittest"));assert_eq!(app.composer.text(),"Unsent steering");
    }
    #[test]
    fn worker_activity_tail_scrolls_at_narrow_width_and_releases_old_message_anchor() {
        let mut app=worker_activity_app();
        let units=(0..16).map(|n|json!({"work_id":"t-scope","title":format!("Worker {n} 日本語"),
            "status":"running","operation_id":format!("worker-{n}")})).collect::<Vec<_>>();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["units"]=json!(units);
        app.journal.add("Waywr","Earlier event");
        app.journal.scroll_anchor_seq=Some(app.journal.messages.last().unwrap().seq);
        let mut terminal=Terminal::new(TestBackend::new(60,24)).unwrap();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        app.journal.scroll_anchor_seq=None;app.journal.scroll_back=0;
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let bottom=app.viewport_top;
        // Even a previously saved message anchor must be cleared when scrolling within live rows.
        app.journal.scroll_anchor_seq=Some(app.journal.messages.last().unwrap().seq);
        app.input(key(KeyCode::PageUp));
        assert!(app.journal.scroll_anchor_seq.is_none());
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        assert!(app.viewport_top<bottom);assert!(app.journal.scroll_back>0);
        app.input(key(KeyCode::PageDown));terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        assert_eq!(app.viewport_top,bottom);
    }
    fn next_topic_snapshot(app: &App) -> Value {
        let mut conversation = app.live.as_ref().unwrap().conversation.clone().unwrap();
        conversation["continuous"]["subjects"].as_array_mut().unwrap().push(
            json!({"subject_id":"discussion-next","task_id":"t-next"}));
        let mut tasks = app.live.as_ref().unwrap().tasks.clone();
        tasks.push(json!({"task_id":"t-next","title":"New migration scope","version":1,
            "state_revision":2,"status":"proposed","run_id":null}));
        json!({"conversation":conversation,"tasks":tasks,"session":{"phase":"running"},"recent_events":[]})
    }
    #[test]
    fn continuous_plan_follows_new_topic_scope_and_rejects_old_detail() {
        let mut app = continuous_app();
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"] =
            json!({"workflows":[{"work_id":"t-scope","status":"delivered"}]});
        let old_detail = app.live.as_ref().unwrap().detail.clone().unwrap();
        app.conversation_subject = "discussion-next".into();
        app.composer.area.insert_str("Unsent steering");
        let draft = serde_json::to_value(app.composer.snapshot()).unwrap();
        let snapshot = next_topic_snapshot(&app);
        app.apply_snapshot(&snapshot);
        assert_eq!(app.selected_title(), "New migration scope");
        assert_eq!(app.selected_status(), "proposed");
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["units"] =
            json!([{"work_id":"t-scope","status":"running"}]);
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        terminal.draw(|frame| app.render(frame, Duration::ZERO)).unwrap();
        let screen = terminal.backend().buffer().content().iter().map(|cell|cell.symbol()).collect::<String>();
        assert!(screen.contains("Development: awaiting scope approval · 0 workers active"));
        assert!(!screen.contains("Development: delivered"));
        assert_eq!(serde_json::to_value(app.composer.snapshot()).unwrap(), draft);
        assert_eq!(app.modal, Modal::None);
        assert!(app.take_intent().is_none());
        app.command("plan");
        assert_eq!(app.pending_primary_task.as_deref(), Some("t-next"));
        assert_eq!(app.take_intent(), Some(LiveIntent::Inspect));
        app.apply_detail(old_detail.clone());
        assert_eq!(app.modal, Modal::None);
        assert!(app.current_scope().is_none());
        let mut detail = old_detail;
        detail["task"] = snapshot["tasks"][1].clone();
        detail["scope"]["base"]["task_id"] = json!("t-next");
        detail["scope"]["base"]["sha256"] = json!("new-sha");
        detail["scope"]["record"]["proposal"]["title"] = json!("New migration scope");
        app.apply_detail(detail);
        assert_eq!(app.modal, Modal::Scope);
        assert_eq!(app.scope_step, 0);
        assert_eq!(app.current_scope().unwrap()["base"]["task_id"], "t-next");
        assert!(app.scope_body().1.contains("New migration scope"));
        assert!(app.take_intent().is_none());
        // Refreshes retain deliberate inspection of the old scope. /plan
        // explicitly returns to the active topic and resets review progress.
        app.input(key(KeyCode::Esc));
        app.select_task(0);
        app.scope_step = 3;
        app.scope_ack = Some("old acknowledgement".into());
        app.apply_snapshot(&snapshot);
        assert_eq!(app.journal.selected_task, 0);
        app.command("plan");
        assert_eq!(app.journal.selected_task, 1);
        assert_eq!(app.scope_step, 0);
        assert!(app.scope_ack.is_none());
        assert_eq!(app.modal, Modal::Scope);
        assert!(app.take_intent().is_none());
    }
    #[test]
    fn continuous_resume_selects_subject_scope_and_new_topic_has_no_old_plan() {
        let mut app = continuous_app();
        let snapshot = next_topic_snapshot(&app);
        app.apply_snapshot(&snapshot);
        assert_eq!(app.journal.selected_task, 0);
        app.restore_conversation_ui(&json!({"schema_version":1,
            "subject_id":"discussion-next","request_revision":false})).unwrap();
        assert_eq!(app.selected_title(), "New migration scope");
        app.command("new");
        app.command("plan");
        assert_eq!(app.modal, Modal::None);
        assert!(app.notice.contains("no saved scope yet"));
        assert!(app.take_intent().is_none());
        assert_eq!(app.live.as_ref().unwrap().tasks.len(), 2);
    }
    #[test]
    fn continuous_new_scope_does_not_replace_an_open_review() {
        let mut app = continuous_app();
        app.open(Modal::Scope);
        app.scope_step = 2;
        app.conversation_subject = "discussion-next".into();
        app.apply_snapshot(&next_topic_snapshot(&app));
        assert_eq!(app.journal.selected_task, 0);
        assert_eq!(app.scope_step, 2);
        app.input(key(KeyCode::Esc));
        app.command("plan");
        assert_eq!(app.selected_title(), "New migration scope");
        assert_eq!(app.scope_step, 0);
        assert_eq!(app.take_intent(), Some(LiveIntent::Inspect));
        assert!(app.take_intent().is_none());
    }
    fn rows(a:&mut App,v:Value){a.live.as_mut().unwrap().conversation.as_mut().unwrap()["continuous"]["queue"]=v;}
    #[test]
    fn live_input_restoration_is_durable_and_does_not_reapply_an_acknowledgement(){
        let input=json!({"source_command":"restore-one","subject_id":"discussion","text":"/exit"});
        let mut a=continuous_app();a.restore_queue_input(input.clone());assert_eq!(a.composer.text(),"/exit");assert!(a.composer.literal_paste);
        let saved=a.conversation_ui_state();let mut reopened=continuous_app();reopened.restore_conversation_ui(&saved).unwrap();reopened.composer.clear();reopened.restore_queue_input(input);
        assert!(reopened.composer.text().is_empty(),"A replayed edit acknowledgement resurrected already consumed input");
    }
    #[test]
    fn live_input_enter_steers_and_alt_enter_queues_a_follow_up(){
        let mut a=continuous_app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["continuous"]["live_input_contract"]=json!(1);rows(&mut a,json!([{"turn_id":"active","subject_id":"discussion","status":"reserved"}]));
        a.input(Event::Paste("Revise the active answer".into()));a.input(key(KeyCode::Enter));
        let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!("Enter queued a later turn instead of live steering")};
        assert_eq!(v["action"],"conversation.steer");assert_eq!(v["payload"]["turn_id"],"active");assert_eq!(v["payload"]["text"],"Revise the active answer");
        a.input(Event::Paste("Then explain the result".into()));a.input(Event::Key(KeyEvent::new(KeyCode::Enter,KeyModifiers::ALT)));
        assert_eq!(a.take_intent(),Some(LiveIntent::SubmitTask("Then explain the result".into())));assert!(a.take_intent().is_none());
    }
    #[test]
    fn live_input_editor_moves_within_multiline_before_recalling_history_and_control_j_adds_newline(){
        let mut a=App::new_live("Fixture".into(),Draft::default());a.input_history.remember(Draft{text:"Older message".into(),..Draft::default()});
        a.composer=Composer::from_draft(Draft{text:"first\nsecond".into(),row:1,col:3,..Draft::default()});
        a.input(key(KeyCode::Up));assert_eq!(a.composer.text(),"first\nsecond");assert_eq!(a.composer.area.cursor().0,0);
        a.input(key(KeyCode::Up));assert_eq!(a.composer.text(),"Older message");
        a.input(key(KeyCode::Down));assert_eq!(a.composer.text(),"first\nsecond");
        a.input(Event::Key(KeyEvent::new(KeyCode::Char('j'),KeyModifiers::CONTROL)));assert_eq!(a.composer.area.lines().len(),3);assert!(a.take_intent().is_none());
    }
    #[test]
    fn live_input_help_explains_the_actual_editor_and_delivery_controls(){
        let mut a=continuous_app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["continuous"]["live_input_contract"]=json!(1);a.open(Modal::Help);
        let body=a.modal_body().1;for control in ["Alt+Enter","Ctrl-J","F7","/thinking","Ctrl-Q"]{assert!(body.contains(control),"Help omits {control}");}
        assert!(a.take_intent().is_none());
    }
    #[test]
    fn live_input_queue_keeps_selection_visible_beyond_long_previews(){
        let mut a=continuous_app();rows(&mut a,Value::Array((0..16).map(|i|json!({"turn_id":format!("later-{i}"),"subject_id":"discussion","status":"queued","text":format!("Input number {i:02} {}", "long draft ".repeat(80))})).collect()));
        a.input(key(KeyCode::F(7)));for _ in 0..12{a.input(key(KeyCode::Down));}
        let mut terminal=Terminal::new(TestBackend::new(120,30)).unwrap();terminal.draw(|frame|a.render(frame,Duration::ZERO)).unwrap();
        let screen=terminal.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();
        assert!(screen.contains("Input number 12"),"Queue selection scrolled outside its visible viewport");
        a.input(key(KeyCode::Char('e')));let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!("Missing queue edit")};assert_eq!(v["payload"]["turn_id"],"later-12");
    }
    #[test]
    fn live_input_queue_can_restore_pending_follow_up_for_editing(){
        let mut a=continuous_app();rows(&mut a,json!([{"turn_id":"later","subject_id":"discussion","status":"queued","text":"Change this later"}]));
        a.input(key(KeyCode::F(7)));a.input(key(KeyCode::Char('e')));
        let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!("Queue has no edit action")};
        assert_eq!(v["action"],"withdraw_queued");assert_eq!(v["payload"]["turn_id"],"later");assert_eq!(v["payload"]["restore_to_editor"],true);
    }
    #[test]
    fn everyday_pause_is_separate_from_leaving(){
        let mut a=continuous_app();for ch in "/pause".chars(){a.input(key(KeyCode::Char(ch)));}a.input(key(KeyCode::Enter));
        assert_eq!(a.take_intent(),Some(LiveIntent::Pause));assert!(!a.detached);assert_eq!(a.modal,Modal::None);
    }
    #[test]
    fn everyday_leave_returns_immediately_from_overlays_and_keeps_draft(){
        for code in [KeyEvent::new(KeyCode::Char('q'),KeyModifiers::CONTROL)] {
            for modal in [Modal::None,Modal::Settings,Modal::ScopeEditor,Modal::QuestionConfirm] {
                let mut a=continuous_app();a.composer.area.insert_str("Unsent draft");a.modal=modal;
                a.input(Event::Key(code));assert!(a.detached,"{modal:?} trapped leave");
                assert_eq!(a.composer.text(),"Unsent draft");
                assert!(!matches!(a.take_intent(),Some(LiveIntent::Cancel|LiveIntent::Pause)));
            }
        }
        for command in ["detach","exit","quit"] {
            let mut a=continuous_app();for ch in format!("/{command}").chars(){a.input(key(KeyCode::Char(ch)));}a.input(key(KeyCode::Enter));assert!(a.detached,"/{command} required confirmation");
        }
        let mut a=continuous_app();a.input(Event::Key(KeyEvent::new(KeyCode::Char('d'),KeyModifiers::CONTROL)));assert!(a.detached);
        let mut a=continuous_app();a.composer.area.insert_str("Draft");a.input(Event::Key(KeyEvent::new(KeyCode::Char('d'),KeyModifiers::CONTROL)));assert!(!a.detached);
    }
    #[test]
    fn everyday_empty_escape_or_control_c_interrupts_only_the_foreground_coordinator(){
        for event in [key(KeyCode::Esc),Event::Key(KeyEvent::new(KeyCode::Char('c'),KeyModifiers::CONTROL))] {
            let mut a=continuous_app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]=json!({"version":1});
            a.conversation_subject="discussion".into();
            rows(&mut a,json!([{"turn_id":"foreign","subject_id":"other","status":"reserved"},
                {"turn_id":"foreground","subject_id":"discussion","status":"reserved"}]));
            a.input(event);assert_eq!(a.modal,Modal::None);
            assert_eq!(a.take_intent(),Some(LiveIntent::Dialogue(json!({"action":"cancel_conversation","payload":{"turn_id":"foreground"}}))));
            a.input(key(KeyCode::Esc));assert!(a.take_intent().is_none(),"Duplicate interruption while stopping");
            assert!(a.take_intent().is_none());assert!(!a.detached);
        }
        let mut a=continuous_app();a.input(Event::Key(KeyEvent::new(KeyCode::Char('c'),KeyModifiers::CONTROL)));
        assert!(a.take_intent().is_none());assert!(!a.detached);
        let mut a=continuous_app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]=json!({"version":1});a.conversation_subject="discussion".into();
        rows(&mut a,json!([{"turn_id":"foreign","subject_id":"other","status":"reserved"}]));a.input(key(KeyCode::Esc));assert!(a.take_intent().is_none());
    }
    #[test]fn continuous_busy_input_queues_and_interrupt_keeps_draft(){
        let mut a=continuous_app();rows(&mut a,json!([{"turn_id":"active","status":"reserved","text":"First"}]));
        assert_eq!(a.action_label(),"Interrupt coordinator");a.input(key(KeyCode::Enter));assert_eq!(a.modal,Modal::None);
        assert_eq!(a.take_intent(),Some(LiveIntent::Dialogue(json!({"action":"cancel_conversation","payload":{"turn_id":"active"}}))));
        a.input(Event::Paste("A later constraint".into()));a.focus=Focus::Activity;a.input(key(KeyCode::Enter));assert_eq!(a.modal,Modal::None);assert_eq!(a.composer.text(),"A later constraint");assert!(a.take_intent().is_none());
        a.focus=Focus::Composer;a.input(key(KeyCode::Enter));assert_eq!(a.take_intent(),Some(LiveIntent::SubmitTask("A later constraint".into())));
        rows(&mut a,json!([{"turn_id":"active","status":"reserved","interrupt_requested":true}]));a.input(key(KeyCode::Enter));assert_eq!(a.modal,Modal::None);assert!(a.take_intent().is_none());
    }
    #[test]fn continuous_queue_release_and_withdraw_are_explicit_and_preserve_composer(){
        let mut a=continuous_app();rows(&mut a,json!([{"turn_id":"old","status":"held","text":"Older constraint"}]));a.input(Event::Paste("Fresh unsent draft".into()));a.input(key(KeyCode::F(7)));assert_eq!(a.modal,Modal::Queue);assert!(a.take_intent().is_none());
        a.input(key(KeyCode::Right));a.input(key(KeyCode::Right));a.input(key(KeyCode::Enter));let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!()};assert_eq!(v["action"],"continue_held");assert_eq!(v["payload"]["turn_ids"],json!(["old"]));assert_eq!(v["payload"]["acknowledged_outcome"]["generation"],2);assert_eq!(a.composer.text(),"Fresh unsent draft");
        a.modal_action=1;a.input(key(KeyCode::Enter));let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!()};assert_eq!(v["action"],"withdraw_queued");assert_eq!(a.composer.text(),"Fresh unsent draft");
        a.input(key(KeyCode::Esc));a.live.as_mut().unwrap().conversation.as_mut().unwrap()["chat"]["status"]=json!("recoverable");assert_eq!(a.message_payload("Only me")["acknowledged_outcome"]["generation"],2);a.input(key(KeyCode::Enter));assert_eq!(a.take_intent(),Some(LiveIntent::SubmitTask("Fresh unsent draft".into())));assert!(a.take_intent().is_none());
    }
    fn formal(a:&mut App){a.live.as_mut().unwrap().conversation.as_mut().unwrap()["actions"]=json!([{"action_id":"formal","kind":"suggest_question_disposition","body":{"subject_id":"discussion","action":{"payload":{"question_id":"q","question_revision":1,"disposition":"answer","answer":"Celsius"}}}}]);}
    #[test]fn continuous_formal_confirmation_is_only_for_formal_decision_and_fences_refresh(){
        let mut a=continuous_app();formal(&mut a);a.input(Event::Paste("Unsent discussion".into()));a.focus=Focus::Activity;a.input(key(KeyCode::Enter));assert_eq!(a.modal,Modal::QuestionConfirm);assert!(a.take_intent().is_none());
        a.input(key(KeyCode::Tab));a.input(key(KeyCode::End));a.input(Event::Paste(" only".into()));a.input(key(KeyCode::Tab));a.input(key(KeyCode::Enter));let Some(LiveIntent::Dialogue(v))=a.take_intent()else{panic!()};assert_eq!(v["action"],"confirm_question_disposition");assert_eq!(v["payload"]["answer"],"Celsius only");assert_eq!(a.composer.text(),"Unsent discussion");
        a.open_question_suggestion();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["actions"][0]["body"]["action"]["payload"]["question_revision"]=json!(2);a.modal_action=2;a.input(key(KeyCode::Enter));assert!(a.take_intent().is_none());assert!(a.dialogue_modal().unwrap().0.contains("changed"));
        a.input(key(KeyCode::Esc));a.focus=Focus::Composer;a.input(key(KeyCode::Enter));assert_eq!(a.take_intent(),Some(LiveIntent::SubmitTask("Unsent discussion".into())));
    }
    #[test]fn continuous_scope_optional_during_discussion_and_local_edit_is_not_mutated(){
        let mut a=continuous_app();rows(&mut a,json!([{"status":"reserved","text":"Unrelated discussion"}]));a.command("plan");assert_eq!(a.modal,Modal::Scope);assert!(a.scope_ready());assert!(a.take_intent().is_none());
        a.input(key(KeyCode::Enter));a.input(key(KeyCode::Enter));a.input(key(KeyCode::Enter));assert!(a.take_intent().is_none());a.input(key(KeyCode::Enter));assert!(matches!(a.take_intent(),Some(LiveIntent::AuthorizeScope(_))));
        a.open(Modal::Scope);a.modal_action=a.scope_actions().iter().position(|(v,_)|*v==ScopeAction::RequestRevision).unwrap();a.input(key(KeyCode::Enter));assert!(a.conversation_revision_request);assert_eq!(a.message_payload("Change the scope")["request_revision"],true);assert!(a.take_intent().is_none());
        let v=a.conversation_ui_state();let mut b=continuous_app();b.restore_conversation_ui(&v).unwrap();assert_eq!(b.conversation_ui_state(),v);assert!(b.restore_conversation_ui(&json!({"schema_version":1,"subject_id":"../other"})).is_err());
    }
    #[test]fn continuous_refresh_preserves_draft_focus_reading_position_and_subject(){
        let mut a=continuous_app();a.input(Event::Paste("Keep draft".into()));a.open(Modal::Inspect);a.modal_scroll=3;a.focus=Focus::Activity;let c=a.live.as_ref().unwrap().conversation.clone().unwrap();let tasks=a.live.as_ref().unwrap().tasks.clone();
        a.apply_snapshot(&json!({"conversation":c,"tasks":tasks,"session":{"phase":"running"},"recent_events":[]}));assert_eq!(a.composer.text(),"Keep draft");assert_eq!(a.modal,Modal::Inspect);assert_eq!(a.modal_scroll,3);assert_eq!(a.focus,Focus::Activity);assert_eq!(a.conversation_subject,"discussion");
    }
    #[test]fn continuous_visible_states_and_inspect_are_bounded_at_terminal_widths(){
        for width in [80,120]{let mut a=continuous_app();rows(&mut a,json!([{"status":"reserved","text":"First"},{"status":"queued","text":"Second"},{"status":"held","text":"Third"}]));a.input(Event::Paste("still composing".into()));let mut t=Terminal::new(TestBackend::new(width,36)).unwrap();t.draw(|f|a.render(f,Duration::ZERO)).unwrap();let screen=t.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();assert!(screen.contains("Draft · not sent"));assert!(screen.contains("Interrupt coordinator"));assert!(screen.contains("Queue F7"));assert!(screen.contains("queued 1 · held 1"));
          a.open(Modal::Inspect);a.live.as_mut().unwrap().conversation.as_mut().unwrap()["budgets"]["conversation"]=json!({"charged":1,"limit":6});t.draw(|f|a.render(f,Duration::ZERO)).unwrap();assert!(a.modal_body().1.contains("1/6 charged"));for _ in 0..100{a.input(key(KeyCode::PageDown));}assert!(a.modal_scroll<=a.modal_scroll_limit);a.input(key(KeyCode::Up));assert!(a.modal_scroll<a.modal_scroll_limit||a.modal_scroll_limit==0);
        }
    }
    #[test]fn continuous_model_save_claim_does_not_supply_application_status(){
        let mut a=continuous_app();a.apply_event(&json!({"seq":100,"type":"conversation.message","body":{"role":"assistant","text":"I saved it"}}));a.apply_event(&json!({"seq":101,"type":"conversation.turn_completed","body":{"action_disposition":"none","scope_saved":false}}));assert!(a.journal.messages.last().unwrap().text.contains("no scope change saved"));assert!(!a.journal.messages.iter().any(|m|m.text.contains("revision 2 saved")));
    }


    #[test]fn ordinary_reply_failure_names_response_problem_and_preserves_input(){
        let mut a=continuous_app();a.apply_event(&json!({"seq":100,"type":"conversation.turn_stopped","body":{"status":"format","reason":"Provider result malformed","recoverable":true}}));
        let text=&a.journal.messages.last().unwrap().text;assert!(text.contains("Reply unavailable · Provider result malformed"));assert!(text.contains("Message saved"));assert!(!text.contains("scope"));assert!(!text.contains("Call counted"));
    }
    #[test]fn continuous_failure_event_supplies_exact_recovery_ack_before_snapshot(){
        let mut a=continuous_app();rows(&mut a,json!([{"turn_id":"failed","status":"reserved"},{"turn_id":"old","status":"queued"}]));a.live.as_mut().unwrap().conversation.as_mut().unwrap()["chat"]["status"]=json!("running");let pause=json!({"generation":3,"outcome_sha256":"exact-outcome","hard":false,"active":true,"recoverable":true});a.apply_event(&json!({"seq":100,"type":"conversation.turn_stopped","body":{"turn_id":"failed","status":"format","recoverable":true,"pause":pause}}));assert_eq!(a.message_payload("Deliberate new Send")["acknowledged_outcome"],pause);assert_eq!(a.queue_rows()[0]["status"],"held");assert_eq!(a.action_label(),"View held messages");a.input(Event::Paste("Deliberate new Send".into()));a.input(key(KeyCode::Enter));assert_eq!(a.take_intent(),Some(LiveIntent::SubmitTask("Deliberate new Send".into())));
    }


    #[test]fn closed_native_settings_never_offer_allowance_or_emit_authorization(){let mut a=continuous_app();let c=a.live.as_mut().unwrap().conversation.as_mut().unwrap();c["chat"]["status"]=json!("blocked");c["chat"]["profile"]["mode"]=json!("native");c["chat"]["reason"]=json!("Builtin plugin boundary unproven");a.open(Modal::Settings);a.input(key(KeyCode::Tab));assert_eq!(a.modal_action,0);let mut t=Terminal::new(TestBackend::new(100,36)).unwrap();t.draw(|f|a.render(f,Duration::ZERO)).unwrap();let screen=t.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();assert!(!screen.contains("Authorize six"));assert!(screen.contains("Native admission unavailable"));a.input(key(KeyCode::Enter));assert!(a.take_intent().is_none());}

    #[test]fn development_policy_drift_resets_review_and_payload_binds_exact_policy(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[],"units":[],"requests":[]});
        let mut detail=app.live.as_ref().unwrap().detail.clone().unwrap();detail["scope"]["operating_policy"]=json!({"project":"/project","main_policy":"checked-fast-forward","concurrency":2});app.apply_detail(detail.clone());
        app.input(key(KeyCode::Enter));app.input(key(KeyCode::Enter));assert!(app.scope_body().1.contains("Operating policy included"));app.input(key(KeyCode::Enter));app.input(key(KeyCode::Enter));assert_eq!(app.scope_step,3);
        detail["scope"]["operating_policy"]["concurrency"]=json!(3);app.apply_detail(detail.clone());assert_eq!(app.scope_step,0);assert!(app.scope_ack.is_none());assert!(app.take_intent().is_none());
        for _ in 0..3{app.input(key(KeyCode::Enter));}app.input(key(KeyCode::Enter));let Some(LiveIntent::AuthorizeScope(payload))=app.take_intent()else{panic!("Missing explicit approval")};assert_eq!(payload["operating_policy_sha256"],waywr_backend::codec::canonical_sha256(&detail["scope"]["operating_policy"]).unwrap());
    }
    #[test]fn ordinary_project_setup_choices_save_without_authorizing_work(){
        let mut app=scope_app();app.provider_section=6;app.project_edit=Some(json!({"version":1,"revision":0,"baseline":"HEAD","baseline_input":"committed","target":"trunk","completion":"checked-local","human_validation":false,"retention":"retire-clean-owned","command_recipes":[]}));
        app.provider_row=1;app.provider_key(KeyEvent::new(KeyCode::Right,KeyModifiers::NONE));assert_eq!(app.project_draft()["baseline_input"],"initialize-files");app.provider_row=4;app.provider_key(KeyEvent::new(KeyCode::Right,KeyModifiers::NONE));assert_eq!(app.project_draft()["human_validation"],true);app.provider_key(KeyEvent::new(KeyCode::Char('s'),KeyModifiers::NONE));
        let Some(LiveIntent::Dialogue(v))=app.take_intent()else{panic!("Missing setup save")};assert_eq!(v["action"],"project.setup");assert_eq!(v["payload"]["expected_revision"],0);assert_eq!(v["payload"]["settings"]["target"],"trunk");assert!(app.take_intent().is_none());
        let text=App::proposal_card_text(&json!({"proposal":{"title":"Existing project","objective":"Normal scope","deliverables":[]},"identity":{"base":{"revision":1}},"policy":{"project_workflow":{"baseline":"HEAD","baseline_input":"committed","target":"trunk","completion":"checked-local","human_validation":true,"retention":"retain","ready":true}}}));assert!(text.contains("trunk"));assert!(text.contains("Human validation before integration: true"));
    }
    #[test]fn development_tasks_history_and_in_scope_steering_are_reachable(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[{"work_id":"t-scope","status":"running","requirement_generation":2,"head":"abc","integration_branch":"waywr/work/integration"}],"units":[{"title":"Storage","status":"submitted","provider":"codex","model":"gpt-6.1-sol","effort":"medium","start_after":["contract"],"obligations":["real production wiring"]}],"requests":[{"log_path":"/project/state/history/request.log","summary_path":"/project/state/history/request.summary.md"}]});
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["context_diagnostics"]=json!([{"role":"coordinator","profile_id":"codex","provider":"codex","model":"gpt-6.1-sol","packet_bytes":42000,"working_target_bytes":98304,"application_memory_limit_bytes":4194304,"token_estimate":{"value":14000},"instructions_bytes":4000,"schema_bytes":2000,"headroom":{"response_tokens":128000},"section_bytes":{"policy":5000},"omissions":[],"retrieval_available":true,"diagnostics_file":"/project/.waywr/state/context-diagnostics.json"}]);
        app.open(Modal::Tasks);let tasks=app.modal_body().1;assert!(tasks.contains("Storage"));assert!(tasks.contains("submitted"));assert!(tasks.contains("real production wiring"));assert!(!tasks.contains("Section bytes"));app.open(Modal::Inspect);assert!(app.modal_body().1.contains("/project/state/history/request.log"));assert!(app.modal_body().1.contains("summary.md"));assert!(app.modal_body().1.contains("Section bytes"));assert!(app.modal_body().1.contains("heuristic; exact tokenizer/native capacity unknown"));assert!(app.modal_body().1.contains("context-diagnostics.json"));
        app.modal=Modal::None;app.command("steer");assert_eq!(app.journal.target,Target::SteerFirst);assert_eq!(app.development_steering_payload("Tighten errors").unwrap()["requirement_generation"],2);assert!(app.take_intent().is_none());
        let dev=&mut app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"];
        dev["workflows"][0]["status"]=json!("delivered");dev["workflows"][0]["obligations"]=json!([{"status":"resolved"}]);dev["units"][0]["status"]=json!("accepted");
        let delivered=app.tasks_text();assert!(delivered.contains("development delivered"));assert!(delivered.contains("0 unresolved obligations"));assert!(delivered.contains("accepted"));assert!(!delivered.contains("workflow not started"));
        assert!(app.development_reply_payload("answer").is_none());
        let dev=&mut app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"];dev["workflows"][0]["status"]=json!("attention");dev["workflows"][0]["revision"]=json!(9);dev["workflows"][0]["question"]=json!({"prompt":"Which format?"});
        app.command("reply");app.input(Event::Paste("Use SVG".into()));app.input(key(KeyCode::Enter));let Some(LiveIntent::Dialogue(v))=app.take_intent()else{panic!("Missing explicit reply")};assert_eq!(v["action"],"development.reply");assert_eq!(v["payload"]["revision"],9);assert_eq!(v["payload"]["text"],"Use SVG");
        app.live.as_mut().unwrap().phase="paused".into();app.input(Event::Paste("A new discussion".into()));app.input(key(KeyCode::Enter));assert!(app.take_intent().is_none());assert_eq!(app.composer.text(),"A new discussion");app.command("resume");let Some(LiveIntent::Dialogue(v))=app.take_intent()else{panic!("Missing explicit resume")};assert_eq!(v["action"],"resume_session");
    }
    #[test]fn convergence_checkout_and_retirement_are_readable_without_sending_actions(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[{"work_id":"t-scope","status":"attention","integration_branch":"waywr/legacy/integration","result_branch":"waywr/ledger-tui-123456abcdef/integration","head":"exact-commit","integration_checkout":{"directory":"/project/.waywr/worktrees/t-scope/integration"}}],"units":[]});
        app.open(Modal::Inspect);let text=app.modal_body().1;assert!(text.contains("waywr/ledger-tui-123456abcdef/integration"));assert!(text.contains("Combined checkout: /project/.waywr/worktrees/t-scope/integration"));assert!(text.contains("main advances only after delivery"));
        app.apply_event(&json!({"seq":999,"type":"development.assignment_retired","body":{"name":"obsolete","retirement":{"reason":"Replaced by exact revision review"}}}));let text=&app.journal.messages.last().unwrap().text;assert!(text.contains("Replaced by exact revision review"));assert!(text.contains("obligations retained"));assert!(app.take_intent().is_none());
    }
    #[test]fn development_events_explain_approval_and_concrete_worker_blocks(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[],"units":[],"requests":[]});
        app.apply_event(&json!({"seq":100,"type":"scope.proposed","body":{"revision":2}}));
        assert!(app.journal.messages.last().unwrap().text.contains("approval is required"));
        app.apply_event(&json!({"seq":101,"type":"development.unit_outcome","body":{"name":"skeleton","status":"failed","worker_report":{"summary":"bwrap: Permission denied"}}}));
        assert!(app.journal.messages.last().unwrap().text.contains("bwrap: Permission denied"));
        app.apply_event(&json!({"seq":102,"type":"development.coordinator_reserved","body":{}}));
        assert!(app.journal.messages.last().unwrap().text.contains("independent workers continue"));
        app.apply_event(&json!({"seq":103,"type":"development.worker_report_retained","body":{"name":"skeleton","worker_report":{"report":{"summary":"Sandbox failed; input retained"}}}}));
        let message=app.journal.messages.last().unwrap();
        assert!(message.text.contains("Sandbox failed"));
        let rendered=app.message_text(message,100).to_string();
        assert!(rendered.contains("Failed"));assert!(!rendered.contains("Done"));
    }
    #[test]fn activity_review_labels_use_disposition_not_failure_words_in_prose(){
        let mut app=continuous_app();
        for (seq,status,changes,stale,label,heading) in [
            (100,"reviewed",false,false,"Passed","Review complete"),
            (101,"reviewed",true,false,"Needs attention","Review needs changes"),
            (102,"failed",false,true,"Needs attention","Review stale"),
            (103,"failed",false,false,"Failed","Review failed"),
        ]{
            app.apply_event(&json!({"seq":seq,"type":"development.review_outcome","body":{"status":status,"review_stale":stale,"review":{"requires_changes":changes,"summary":"No findings. Failed-save preservation and cancelled-operation behavior match scope; combined checks remain outstanding."}}}));
            let m=app.journal.messages.last().unwrap();let rendered=app.message_text(m,180);let header=rendered.lines[0].to_string();
            assert!(header.contains(label),"{header}");assert!(header.contains(heading),"{header}");
            if status=="reviewed"{assert!(!header.contains("Failed"));}
            assert!(m.text.contains("combined checks remain outstanding"));assert!(app.take_intent().is_none());
        }
    }
    #[test]fn activity_checks_and_other_development_events_ignore_prose_keywords(){
        let mut app=continuous_app();
        app.apply_event(&json!({"seq":100,"type":"development.combined_checks","body":{"checks":{"result":{"passed":true}}}}));
        assert!(app.message_text(app.journal.messages.last().unwrap(),120).lines[0].to_string().contains("Passed"));
        app.apply_event(&json!({"seq":101,"type":"development.unit_integrated","body":{"title":"Preserve failed-save and uncertain-input handling"}}));
        let header=app.message_text(app.journal.messages.last().unwrap(),160).lines[0].to_string();assert!(header.contains("Activity"));assert!(!header.contains("Uncertain"));assert!(!header.contains("Failed"));
        app.apply_event(&json!({"seq":102,"type":"development.combined_checks","body":{"checks":{"result":{"passed":false}}}}));
        assert!(app.message_text(app.journal.messages.last().unwrap(),120).lines[0].to_string().contains("Failed"));
    }
    #[test]fn activity_cached_legacy_review_rehydrates_from_exact_event_without_duplication(){
        let mut app=continuous_app();let legacy=json!({"seq":100,"speaker":"Waywr","text":"Review reviewed · failed-save preservation is correct","task_id":"t-scope"});
        let message:super::super::scenario::Message=serde_json::from_value(legacy).unwrap();assert!(message.activity.is_none());
        let header=app.message_text(&message,150).lines[0].to_string();assert!(header.contains("Activity"));assert!(!header.contains("Failed"));
        app.journal.messages.push(message);app.live.as_mut().unwrap().last_seq=100;let count=app.journal.messages.len();
        let event=json!({"seq":100,"type":"development.review_outcome","body":{"status":"reviewed","review":{"requires_changes":false,"summary":"failed-save preservation is correct"}}});
        app.apply_event(&event);app.apply_event(&event);assert_eq!(app.journal.messages.len(),count);
        let header=app.message_text(app.journal.messages.last().unwrap(),150).lines[0].to_string();assert!(header.contains("Passed"));assert!(header.contains("Review complete"));assert!(app.take_intent().is_none());
        let saved=serde_json::to_value(app.journal.messages.last().unwrap()).unwrap();let restored:super::super::scenario::Message=serde_json::from_value(saved).unwrap();assert_eq!(restored.activity,Some(super::super::scenario::ActivityStatus::Success));
    }
    #[test]fn activity_bound_new_topic_caption_follows_scope_without_retargeting_input(){
        let mut app=continuous_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]=json!({"version":1});app.journal.connection="connected".into();app.journal.target=Target::NewTask;app.composer.area.insert_str("Unsent follow-up");
        let subject=app.conversation_subject.clone();let input=app.message_payload("Unsent follow-up");
        let mut terminal=Terminal::new(TestBackend::new(160,36)).unwrap();terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let screen=terminal.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();assert!(screen.contains("To: Coordinator conversation · scope: Weather scope"));assert!(!screen.contains("no selected scope"));
        assert_eq!(app.conversation_subject,subject);assert_eq!(app.journal.target,Target::NewTask);assert_eq!(app.composer.text(),"Unsent follow-up");assert_eq!(app.message_payload("Unsent follow-up"),input);assert!(app.take_intent().is_none());
        app.conversation_subject="new-unbound-topic".into();terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();let screen=terminal.backend().buffer().content().iter().map(|c|c.symbol()).collect::<String>();assert!(screen.contains("New topic / proposal discussion (no selected scope)"));
    }
    #[test]fn restored_paused_view_shows_session_phase_without_resuming(){
        let mut app=scope_app();
        let conversation=app.live.as_ref().unwrap().conversation.clone().unwrap();
        let tasks=app.live.as_ref().unwrap().tasks.clone();
        app.apply_snapshot(&json!({"session":{"phase":"paused"},"conversation":conversation,"tasks":tasks,"recent_events":[]}));
        assert!(app.take_intent().is_none());
        let mut terminal=Terminal::new(TestBackend::new(120,36)).unwrap();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let text=terminal.backend().buffer().content().iter().map(|cell|cell.symbol()).collect::<String>();
        assert!(text.contains("Session paused"));
        assert_eq!(app.live.as_ref().unwrap().phase,"paused");
    }
    #[test]fn development_milestones_are_readable_while_internal_events_keep_the_cursor(){
        let mut app=scope_app();app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]=json!({"workflows":[],"units":[],"requests":[]});
        let before=app.journal.messages.len();
        app.apply_event(&json!({"seq":100,"type":"development.knowledge_updated","body":{}}));
        assert_eq!(app.journal.messages.len(),before);assert_eq!(app.live.as_ref().unwrap().last_seq,100);
        app.apply_event(&json!({"seq":101,"type":"development.unit_integrated","body":{"title":"JSON persistence"}}));
        assert_eq!(app.journal.messages.last().unwrap().text,"Task integrated · JSON persistence");
        app.apply_event(&json!({"seq":102,"type":"development.combined_checks","body":{"checks":{"result":{"passed":false}}}}));
        assert!(app.journal.messages.last().unwrap().text.contains("checks failed"));
        app.apply_event(&json!({"seq":103,"type":"development.delivered","body":{"main_branch":"release"}}));
        assert!(app.journal.messages.last().unwrap().text.contains("target release advanced"));
        let count=app.journal.messages.len();app.apply_event(&json!({"seq":103,"type":"development.delivered","body":{}}));assert_eq!(app.journal.messages.len(),count);
    }

}

include!("provider_view.rs");
include!("provider_picker.rs");
include!("provider_activation.rs");

include!("coordination_view.rs");

include!("project_view.rs");

include!("candidate_view.rs");
