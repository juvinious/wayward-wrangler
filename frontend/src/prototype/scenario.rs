//! Isolated, scripted presentation data. This module cannot contact the real service.
use super::editor::Draft;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub const SOURCE_HASH: &str = "021e66e9756a8719d3e384c5adb9e5ce0c5d8cdbe089a5595b6a538333d1c8eb";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum First {
    Empty,
    PendingDelivery,
    Coordinator,
    Suggested,
    Proposed,
    Authorized,
    Drafting,
    Critiquing,
    Question,
    AnswerUnknown,
    Revising,
    Review,
    Approved,
    Rejected,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Second {
    None,
    Proposed,
    AuthorizedQueued,
    Active,
    Recovering,
    Recovered,
    Uncertain,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Coordinator,
    NewTask,
    SteerFirst,
    SteerSecond,
    ReplyFirst,
}
impl Target {
    pub fn key(self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::NewTask => "new-task",
            Self::SteerFirst => "steer-first",
            Self::SteerSecond => "steer-second",
            Self::ReplyFirst => "reply-first",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub at: Option<f64>,
    pub seq: u64,
    pub speaker: String,
    pub text: String,
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub task_version: u32,
    #[serde(default)]
    pub simulated: bool,
    #[serde(default)]
    pub source_sha256: String,
    /// Display metadata from the durable event, never inferred from model prose.
    #[serde(default)]
    pub activity: Option<ActivityStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ActivityStatus { Activity, Success, Failure, Attention, Uncertain, Cancelled }

impl Message {
    pub fn is_delivery_transition(&self) -> bool {
        self.speaker == "Waywr"
            && (self.text.starts_with("Sending…") || self.text.starts_with("Request saved ·"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Journal {
    pub schema: u8,
    pub scenario: String,
    pub source_hash: String,
    pub first: First,
    pub second: Second,
    pub first_request: String,
    pub second_request: String,
    pub answer: Option<String>,
    pub candidate: Option<String>,
    pub candidate_sha256: Option<String>,
    pub reviewed_sha256: Option<String>,
    pub approved_sha256: Option<String>,
    pub answer_delivery: String,
    pub connection: String,
    pub first_calls: u8,
    pub second_calls: u8,
    pub coordinator_calls: u8,
    pub applied_commands: Vec<String>,
    pub messages: Vec<Message>,
    pub next_seq: u64,
    pub target: Target,
    pub selected_task: usize,
    pub scroll_back: u16,
    #[serde(default)]
    pub scroll_anchor_seq: Option<u64>,
    #[serde(default)]
    pub scroll_anchor_line: u16,
    pub unread: u16,
    pub drawer: bool,
    pub drafts: BTreeMap<String, Draft>,
    pub theme: String,
    pub effects: String,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            schema: 1,
            scenario: "decision-log".into(),
            source_hash: SOURCE_HASH.into(),
            first: First::Empty,
            second: Second::None,
            first_request: String::new(),
            second_request: String::new(),
            answer: None,
            candidate: None,
            candidate_sha256: None,
            reviewed_sha256: None,
            approved_sha256: None,
            answer_delivery: "not sent".into(),
            connection: "connected".into(),
            first_calls: 0,
            second_calls: 0,
            coordinator_calls: 0,
            applied_commands: Vec::new(),
            messages: Vec::new(),
            next_seq: 1,
            target: Target::Coordinator,
            selected_task: 0,
            scroll_back: 0,
            scroll_anchor_seq: None,
            scroll_anchor_line: 0,
            unread: 0,
            drawer: false,
            drafts: BTreeMap::new(),
            theme: "dark".into(),
            effects: "subtle".into(),
        }
    }
}

impl Journal {
    pub fn add(&mut self, speaker: &str, text: impl Into<String>) {
        let task_id = "decision-log-addendum";
        let task_version = self.first as u32;
        let msg = Message {
            at: Some(waywr_backend::execution::now()),
            seq: self.next_seq,
            speaker: speaker.into(),
            text: text.into(),
            task_id: task_id.into(),
            task_version,
            simulated: true,
            source_sha256: SOURCE_HASH.into(),
            activity: None,
        };
        self.next_seq += 1;
        if (self.scroll_back > 0 || self.scroll_anchor_seq.is_some())
            && !msg.is_delivery_transition()
        {
            self.unread = self.unread.saturating_add(1);
        }
        self.messages.push(msg);
        while self.messages.len() > 200
            || self.messages.iter().map(|m| m.text.len()).sum::<usize>() > 2 * 1024 * 1024
        {
            self.messages.remove(0);
        }
    }
    fn add_second(&mut self, speaker: &str, text: impl Into<String>) {
        self.add(speaker, text);
        if let Some(last) = self.messages.last_mut() {
            last.task_id = "manual-walkthrough".into();
            last.task_version = self.second as u32;
        }
    }

    pub fn submit(&mut self, text: String) -> &'static str {
        if self.connection != "connected" {
            return "Disconnected: draft preserved; reconnect before sending";
        }
        match self.target {
            Target::Coordinator => {
                self.add("You · coordinator", &text);
                if self.first == First::Empty {
                    self.first_request = text;
                    self.first = First::PendingDelivery;
                    self.coordinator_calls += 1;
                    self.add(
                        "Waywr",
                        "Sending… not yet saved by the scripted coordinator. No task authorized.",
                    );
                } else {
                    self.add("Coordinator · SCRIPTED", "I can discuss only the scripted Decision Log choices in this preview. Choose New task for the separate walkthrough proposal.");
                }
                "Conversation saved in the isolated demo"
            }
            Target::NewTask => {
                if self.second != Second::None {
                    return "This scripted demonstration has one additional task slot; draft preserved";
                }
                self.add_second("You · new task", &text);
                self.second_request = text;
                self.second = Second::Proposed;
                self.add_second("Coordinator · SCRIPTED", "Proposed: prepare a manual walkthrough for the Decision Log addendum. Review its scope before authorizing. It will queue behind the first task.");
                self.target = Target::Coordinator;
                "Second task proposal ready; authorization is separate"
            }
            Target::SteerFirst | Target::SteerSecond | Target::ReplyFirst => {
                "Steering is unavailable in this demo; your draft is preserved"
            }
        }
    }

    pub fn suggest(&mut self) -> bool {
        if self.first != First::Coordinator {
            return false;
        }
        self.first = First::Suggested;
        self.add("Coordinator · SCRIPTED", "I can propose a bounded specification addendum. The approved free in-place editing choice remains inherited. Alternatives minimum, supersession correction and cross-tab behavior remain open. Use the visible Create task proposal action to review scope.");
        true
    }
    pub fn propose_first(&mut self) -> bool {
        if self.first != First::Suggested {
            return false;
        }
        self.first = First::Proposed;
        self.add(
            "Waywr",
            "Scope ready to review · no worker call yet. [Review scope]",
        );
        true
    }
    pub fn authorize(&mut self, task: usize) -> bool {
        if task == 0 && self.first == First::Proposed {
            self.first = First::Authorized;
            self.add(
                "Waywr",
                "Clarify Decision Log choices · authorized; scripted work queued.",
            );
            return true;
        }
        if task == 1 && self.second == Second::Proposed {
            self.second = Second::AuthorizedQueued;
            self.add_second(
                "Waywr",
                "Manual walkthrough · authorized, queued behind the first task.",
            );
            return true;
        }
        false
    }
    pub fn advance(&mut self) -> bool {
        match self.first {
            First::PendingDelivery => {
                self.first = First::Coordinator;
                self.add(
                    "Waywr",
                    "Request saved · scripted coordinator working. No worker call authorized.",
                );
            }
            First::Coordinator => return self.suggest(),
            First::Authorized => {
                self.first = First::Drafting;
                self.first_calls += 1;
                self.add(
                    "Waywr",
                    "Drafting · fake author call 1/5 started. You can keep typing.",
                );
            }
            First::Drafting => {
                self.first = First::Critiquing;
                self.first_calls += 1;
                self.add(
                    "Waywr",
                    "Draft retained · fake independent critique call 2/5 started.",
                );
            }
            First::Critiquing => {
                self.first = First::Question;
                self.add("Waywr", "Substantive question ready: may a decision have zero alternatives? [Answer question]");
            }
            First::AnswerUnknown => {
                self.connection = "connected".into();
                self.answer_delivery = "saved".into();
                self.first = First::Revising;
                self.first_calls += 1;
                self.add("Waywr", "Reconnected · saved answer found under the same demo command ID; consumed once. Fake revision call 3/5 started.");
            }
            First::Revising => {
                self.first = First::Review;
                let answer = self.answer.clone().unwrap_or_default();
                let text = format!(
                    "# Project Decision Log — scripted addendum candidate\n\nSource: approved provisional specification, SHA-256 {SOURCE_HASH}.\nAccepted: title, rationale and alternatives can be edited freely in place; prior values are not retained.\n\n## Alternatives minimum\nRecorded answer: {answer}\n{}\n\n## Example record\n```json\n{{\"title\":\"Use local storage\",\"rationale\":\"Available offline\",\"alternatives\":[\"Store in memory\"]}}\n```\n\n## Still unresolved\n- C1-3/C2-2: whether supersession can be reversed or retargeted, and with what timestamp rule.\n- C2-3: behavior when two browser tabs hold conflicting local state.\n\nThis is a SCRIPTED candidate, not a browser application. It received no further independent critique. Approval here is demonstration-only.\n",
                    if answer == "Allow none" {
                        "A decision may have zero alternatives; display ‘No alternatives recorded’."
                    } else if answer == "Require one" {
                        "A decision needs at least one nonblank alternative."
                    } else {
                        "The free-form answer is quoted but not interpreted by this fixture; alternatives remain unresolved."
                    }
                );
                let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
                self.candidate = Some(text);
                self.candidate_sha256 = Some(hash);
                self.add("Waywr", "Addendum candidate ready · two topics unresolved; no further independent critique. [Review candidate]");
            }
            First::Approved | First::Rejected => {
                match self.second {
                    Second::AuthorizedQueued => {
                        self.second = Second::Active;
                        self.second_calls += 1;
                        self.add_second("Waywr", "Manual walkthrough · fake worker call 1/5 started in the scripted queue.");
                    }
                    Second::Active => {
                        self.second = Second::Recovering;
                        self.add_second("Waywr", "Workspace unavailable. Reconciling the owned scripted worker (attempt 1 of 3); no new call.");
                    }
                    Second::Recovering => {
                        self.second = Second::Recovered;
                        self.add_second("Waywr", "Workspace routing restored from the same simulated worker identity and receipt; call count remains 1.");
                    }
                    Second::Recovered => {
                        self.second = Second::Uncertain;
                        self.add_second("Waywr", "Worker report missing: execution uncertain. Task stopped; request, receipt trace and one call remain inspectable. No automatic retry. [Inspect evidence]");
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }

    pub fn answer(&mut self, value: String) -> bool {
        if self.first != First::Question || value.trim().is_empty() {
            return false;
        }
        if self
            .applied_commands
            .iter()
            .any(|s| s == "sim:first:question-1")
        {
            return false;
        }
        self.applied_commands.push("sim:first:question-1".into());
        self.answer = Some(value.clone());
        self.answer_delivery = "saved; reply lost".into();
        self.first = First::AnswerUnknown;
        self.connection = "disconnected".into();
        self.add("Waywr", format!("Answer saved in demo journal: {value}. Reply lost; checking the same command ID. No worker call for this answer."));
        true
    }
    pub fn review(&mut self) -> bool {
        let Some(hash) = self.candidate_sha256.as_ref() else {
            return false;
        };
        let Some(candidate) = self.candidate.as_ref() else {
            return false;
        };
        if format!("{:x}", Sha256::digest(candidate.as_bytes())) != *hash {
            return false;
        }
        self.reviewed_sha256 = Some(hash.clone());
        true
    }
    pub fn approve(&mut self) -> bool {
        if self.first != First::Review || self.reviewed_sha256 != self.candidate_sha256 {
            return false;
        }
        let (Some(candidate), Some(expected)) = (&self.candidate, &self.candidate_sha256) else {
            return false;
        };
        if format!("{:x}", Sha256::digest(candidate.as_bytes())) != *expected {
            return false;
        }
        let Some(hash) = self.candidate_sha256.clone() else {
            return false;
        };
        if self
            .applied_commands
            .iter()
            .any(|s| s == "sim:first:approval")
        {
            return false;
        }
        self.applied_commands.push("sim:first:approval".into());
        self.approved_sha256 = Some(hash);
        self.first = First::Approved;
        self.add("Waywr", "Scripted addendum approved for this demonstration only. Unresolved supersession and cross-tab choices remain; no browser app was built.");
        true
    }
    pub fn reject(&mut self, reason: &str) -> bool {
        if self.first != First::Review || reason.trim().is_empty() {
            return false;
        }
        self.first = First::Rejected;
        self.add(
            "Waywr",
            format!(
                "Scripted candidate rejected: {}. No revision call was started.",
                reason.trim()
            ),
        );
        true
    }
    pub fn cancel(&mut self, task: usize) -> bool {
        if task == 0
            && !matches!(
                self.first,
                First::Empty | First::Approved | First::Rejected | First::Cancelled
            )
        {
            self.first = First::Cancelled;
            self.add("Waywr","Clarify Decision Log choices cancelled in the isolated demonstration. Fake calls and prior answer evidence remain inspectable.");
            return true;
        }
        if task == 1
            && !matches!(
                self.second,
                Second::None | Second::Uncertain | Second::Cancelled
            )
        {
            self.second = Second::Cancelled;
            self.add_second("Waywr","Manual walkthrough cancelled in the isolated demonstration. The other task is unchanged.");
            return true;
        }
        false
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let file = dir.join("scripted-journal.json");
        let tmp = dir.join(".scripted-journal.tmp");
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut out = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        out.write_all(&bytes)?;
        out.sync_all()?;
        fs::rename(tmp, file)?;
        fs::File::open(dir)?.sync_all()?;
        Ok(())
    }
    pub fn load(dir: &Path) -> io::Result<Self> {
        let file = dir.join("scripted-journal.json");
        if !file.exists() {
            return Ok(Self::default());
        }
        let value: Self = serde_json::from_slice(&fs::read(file)?)?;
        if value.schema != 1 || value.scenario != "decision-log" || value.source_hash != SOURCE_HASH
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown scripted journal version or source; use a fresh data directory",
            ));
        }
        if let (Some(text), Some(hash)) = (&value.candidate, &value.candidate_sha256) {
            if format!("{:x}", Sha256::digest(text.as_bytes())) != *hash {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Scripted candidate bytes no longer match saved hash",
                ));
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answer_reconnect_and_approval_are_once_and_bound() {
        let mut j = Journal::default();
        j.submit("Clarify alternatives".into());
        assert_eq!(j.first, First::PendingDelivery);
        j.advance();
        j.advance();
        j.propose_first();
        j.authorize(0);
        j.advance();
        j.advance();
        j.advance();
        assert_eq!(j.first_calls, 2);
        assert!(j.answer("Allow none".into()));
        assert!(!j.answer("Require one".into()));
        assert_eq!(j.first_calls, 2);
        j.advance();
        j.advance();
        assert_eq!(j.first_calls, 3);
        assert_eq!(j.first, First::Review);
        assert!(!j.approve());
        assert!(j.review());
        assert!(j.approve());
        assert!(!j.approve());
        assert_eq!(j.approved_sha256, j.candidate_sha256);
    }
    #[test]
    fn candidate_change_refuses_review_and_second_recovery_never_retries() {
        let mut j = Journal::default();
        j.submit("Please clarify".into());
        j.advance();
        j.advance();
        j.propose_first();
        j.authorize(0);
        j.target = Target::NewTask;
        j.submit("Walkthrough".into());
        j.authorize(1);
        j.advance();
        j.advance();
        j.advance();
        j.answer("Require one".into());
        j.advance();
        j.advance();
        j.candidate.as_mut().unwrap().push('!');
        assert!(!j.review());
        assert!(!j.approve());
        j.candidate.as_mut().unwrap().pop();
        j.review();
        j.approve();
        for _ in 0..4 {
            assert!(j.advance());
        }
        assert_eq!(j.second, Second::Uncertain);
        assert_eq!(j.second_calls, 1);
        assert!(!j.advance());
    }
    #[test]
    fn journal_reopen_preserves_answer_and_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = Journal::default();
        j.submit("Clarify".into());
        j.advance();
        j.advance();
        j.propose_first();
        j.authorize(0);
        j.advance();
        j.advance();
        j.advance();
        j.answer("Allow none".into());
        j.save(tmp.path()).unwrap();
        let mut loaded = Journal::load(tmp.path()).unwrap();
        assert_eq!(loaded.connection, "disconnected");
        loaded.advance();
        assert_eq!(loaded.first_calls, 3);
        assert_eq!(loaded.answer.as_deref(), Some("Allow none"));
    }
    #[test]
    fn targeted_cancel_preserves_other_task_and_second_provenance() {
        let mut j = Journal::default();
        j.submit("Clarify alternatives".into());
        j.advance();
        j.advance();
        j.propose_first();
        j.authorize(0);
        j.target = Target::NewTask;
        j.submit("Write walkthrough".into());
        j.authorize(1);
        let before = j.first;
        assert!(j.cancel(1));
        assert_eq!(j.first, before);
        assert_eq!(j.second, Second::Cancelled);
        assert!(!j.cancel(1));
        assert_eq!(j.messages.last().unwrap().task_id, "manual-walkthrough");
        assert!(j
            .messages
            .iter()
            .all(|m| m.simulated && m.source_sha256 == SOURCE_HASH));
    }
    #[test]
    fn candidate_changed_after_read_cannot_be_approved() {
        let mut j = Journal::default();
        j.submit("Clarify".into());
        j.advance();
        j.advance();
        j.propose_first();
        j.authorize(0);
        j.advance();
        j.advance();
        j.advance();
        j.answer("Allow none".into());
        j.advance();
        j.advance();
        assert!(j.review());
        j.candidate
            .as_mut()
            .unwrap()
            .push_str("\nchanged after read");
        assert!(!j.approve());
        assert_eq!(j.approved_sha256, None);
    }
}
