//! Project-local composer recall. These entries carry no execution authority.
use super::editor::{Draft, MAX_DRAFT};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, Read},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use waywr_backend::execution::{FileLock, write_bytes};

const MAX_ENTRIES: usize = 200;
const MAX_FILE: usize = 1024 * 1024;
static ENTRY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    id: String,
    draft: Draft,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    schema_version: u8,
    entries: Vec<Entry>,
}

#[derive(Default)]
pub struct InputHistory {
    entries: Vec<Entry>,
    pending: Vec<Entry>,
    cursor: Option<usize>,
    original: Option<Draft>,
    edits: BTreeMap<usize, Draft>,
}

impl InputHistory {
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take((MAX_FILE + 1) as u64).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_FILE {
            return Err(io::Error::other(
                "Command history exceeds 1 MiB; existing file preserved",
            ));
        }
        let saved: Saved = serde_json::from_slice(&bytes)?;
        let mut ids = HashSet::new();
        if saved.schema_version != 1
            || saved.entries.len() > MAX_ENTRIES
            || saved.entries.iter().any(|entry| {
                entry.id.is_empty()
                    || entry.id.len() > 128
                    || !ids.insert(entry.id.clone())
                    || entry.draft.text.trim().is_empty()
                    || entry.draft.text.len() > MAX_DRAFT
            })
        {
            return Err(io::Error::other(
                "Invalid command history; existing file preserved",
            ));
        }
        Ok(Self {
            entries: saved.entries,
            ..Self::default()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn recent(&self, index: usize) -> Option<&Draft> {
        self.entries
            .iter()
            .rev()
            .nth(index)
            .map(|entry| &entry.draft)
    }
    pub fn reset_navigation(&mut self) {
        self.cursor = None;
        self.original = None;
        self.edits.clear();
    }
    pub fn is_recalling(&self)->bool{self.cursor.is_some()}
    pub fn take_original(&mut self) -> Option<Draft> {
        let original = self.original.take();
        self.reset_navigation();
        original
    }
    pub fn draft_to_save(&self, current: Draft) -> Draft {
        self.original.clone().unwrap_or(current)
    }
    pub fn remember(&mut self, draft: Draft) {
        if draft.text.trim().is_empty() || draft.text.len() > MAX_DRAFT {
            return;
        }
        // Consecutive duplicate submissions need only one recall slot. Preserve
        // literal-paste semantics, so a slash message cannot become a command.
        if self.entries.last().is_none_or(|entry| {
            entry.draft.text != draft.text || entry.draft.literal_paste != draft.literal_paste
        }) {
            let id = format!(
                "{:x}-{:x}-{:x}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
                std::process::id(),
                ENTRY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            );
            self.seed(id, draft);
        }
        self.reset_navigation();
    }
    pub fn seed(&mut self, id: String, draft: Draft) {
        if draft.text.trim().is_empty() || draft.text.len() > MAX_DRAFT {
            return;
        }
        if self.entries.iter().any(|entry| entry.id == id) {
            return;
        }
        let entry = Entry { id, draft };
        self.pending.push(entry.clone());
        self.entries.push(entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
    }
    pub fn recall(&mut self, older: bool, current: Draft) -> Option<Draft> {
        if self.entries.is_empty() || !older && self.cursor.is_none() {
            return None;
        }
        if let Some(index) = self.cursor {
            self.edits.insert(index, current);
        } else {
            self.original = Some(current);
        }
        let index = match self.cursor {
            None => self.entries.len() - 1,
            Some(index) if older => index.saturating_sub(1),
            Some(index) => index + 1,
        };
        if index == self.entries.len() {
            self.cursor = None;
            self.edits.clear();
            return self.original.take();
        }
        self.cursor = Some(index);
        Some(
            self.edits
                .get(&index)
                .unwrap_or(&self.entries[index].draft)
                .clone(),
        )
    }
    pub fn select_recent(&mut self, index: usize, current: Draft) -> Option<Draft> {
        let selected = self.entries.len().checked_sub(index + 1)?;
        if let Some(index) = self.cursor {
            self.edits.insert(index, current);
        } else {
            self.original = Some(current);
        }
        self.cursor = Some(selected);
        Some(
            self.edits
                .get(&selected)
                .unwrap_or(&self.entries[selected].draft)
                .clone(),
        )
    }
    pub fn save(&mut self, path: &Path) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let _lock = FileLock::exclusive(&path.with_extension("lock")).map_err(io::Error::other)?;
        // Merge only new entries while locked; another attached UI's history
        // must not be overwritten by our older in-memory snapshot.
        let mut saved = Self::load(path)?.entries;
        let mut ids: HashSet<String> = saved.iter().map(|entry| entry.id.clone()).collect();
        saved.extend(
            self.pending
                .iter()
                .filter(|entry| ids.insert(entry.id.clone()))
                .cloned(),
        );
        if saved.len() > MAX_ENTRIES {
            saved.drain(..saved.len() - MAX_ENTRIES);
        }
        let mut record = Saved {
            schema_version: 1,
            entries: saved,
        };
        let bytes = loop {
            let bytes = serde_json::to_vec(&record)?;
            if bytes.len() <= MAX_FILE {
                break bytes;
            }
            record.entries.remove(0);
        };
        write_bytes(path, &bytes, false).map_err(io::Error::other)?;
        self.pending.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft(text: &str) -> Draft {
        Draft {
            text: text.into(),
            col: text.len(),
            ..Draft::default()
        }
    }
    #[test]
    fn recall_restores_original_and_edited_entries_without_wrapping() {
        let mut history = InputHistory::default();
        history.remember(draft("first"));
        history.remember(draft("second"));
        let mut original = draft("draft\n続き");
        original.row = 1;
        original.col = 2;
        original.multiline = true;
        original.literal_paste = true;
        assert_eq!(
            history.recall(true, original.clone()).unwrap().text,
            "second"
        );
        assert_eq!(history.draft_to_save(draft("second")).text, original.text);
        assert_eq!(
            history.recall(true, draft("edited second")).unwrap().text,
            "first"
        );
        assert_eq!(
            history.recall(true, draft("edited first")).unwrap().text,
            "edited first"
        );
        assert_eq!(
            history.recall(false, draft("edited first")).unwrap().text,
            "edited second"
        );
        assert_eq!(
            serde_json::to_value(history.recall(false, draft("edited second")).unwrap()).unwrap(),
            serde_json::to_value(original.clone()).unwrap()
        );
        assert!(history.recall(false, original).is_none());
    }
    #[test]
    fn restart_concurrent_writers_and_literal_slash_semantics_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("command-history.json");
        let mut a = InputHistory::load(&path).unwrap();
        let mut b = InputHistory::load(&path).unwrap();
        a.remember(draft("/plan"));
        a.save(&path).unwrap();
        let mut pasted = draft("/approve");
        pasted.literal_paste = true;
        b.remember(pasted);
        b.save(&path).unwrap();
        a.remember(draft("third"));
        a.save(&path).unwrap();
        let restored = InputHistory::load(&path).unwrap();
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.recent(0).unwrap().text, "third");
        assert!(restored.recent(1).unwrap().literal_paste);
        assert!(!restored.recent(2).unwrap().literal_paste);
        let other = InputHistory::load(&temp.path().join("other-project/history.json")).unwrap();
        assert!(other.is_empty());
    }
    #[test]
    fn malformed_existing_file_is_never_replaced_and_history_is_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("command-history.json");
        let mut history = InputHistory::default();
        history.remember(draft("kept"));
        fs::write(&path, b"unexpected user content").unwrap();
        assert!(history.save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"unexpected user content");
        fs::remove_file(&path).unwrap();
        for n in 0..MAX_ENTRIES + 5 {
            history.remember(draft(&format!("entry {n}")));
        }
        history.save(&path).unwrap();
        assert_eq!(InputHistory::load(&path).unwrap().len(), MAX_ENTRIES);
        for n in 0..40 {
            history.remember(draft(&format!("{n}{}", "\0".repeat(MAX_DRAFT - 10))));
        }
        history.save(&path).unwrap();
        assert!(fs::metadata(&path).unwrap().len() <= MAX_FILE as u64);
        assert!(!InputHistory::load(&path).unwrap().is_empty());
    }
}
