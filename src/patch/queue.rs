use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PatchQueueEntry {
    pub session_id: String,
    pub task_label: String,
    pub patch_content: Option<String>,
    pub status: EntryStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryStatus {
    Pending,      // waiting for patch from Jules
    Ready,        // patch fetched, ready to apply
    _Applying,     // currently being applied
    Applied,      // successfully applied
    Conflicted,   // conflict detected, resolution in progress
    _Failed(String),
}

pub struct PatchQueue {
    entries: VecDeque<PatchQueueEntry>,
}

impl PatchQueue {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, session_id: &str, task_label: &str) {
        self.entries.push_back(PatchQueueEntry {
            session_id: session_id.to_string(),
            task_label: task_label.to_string(),
            patch_content: None,
            status: EntryStatus::Pending,
        });
    }

    pub fn set_patch(&mut self, session_id: &str, patch: String) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.session_id == session_id) {
            entry.patch_content = Some(patch);
            entry.status = EntryStatus::Ready;
        }
    }

    pub fn _next_ready(&mut self) -> Option<&mut PatchQueueEntry> {
        self.entries
            .iter_mut()
            .find(|e| e.status == EntryStatus::Ready)
    }

    pub fn mark_applied(&mut self, session_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.session_id == session_id) {
            entry.status = EntryStatus::Applied;
        }
    }

    pub fn mark_conflicted(&mut self, session_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.session_id == session_id) {
            entry.status = EntryStatus::Conflicted;
        }
    }

    pub fn resolve_conflict(&mut self, session_id: &str, resolved_patch: String) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.session_id == session_id) {
            entry.patch_content = Some(resolved_patch);
            entry.status = EntryStatus::Ready;
        }
    }

    pub fn reorder(&mut self, new_order: &[String]) {
        let mut reordered = VecDeque::new();
        for id in new_order {
            if let Some(pos) = self.entries.iter().position(|e| &e.session_id == id) {
                reordered.push_back(self.entries.remove(pos).unwrap());
            }
        }
        // Append any remaining entries not in new_order
        reordered.extend(self.entries.drain(..));
        self.entries = reordered;
    }

    pub fn all(&self) -> &VecDeque<PatchQueueEntry> {
        &self.entries
    }

    pub fn pending_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, EntryStatus::Pending | EntryStatus::Ready))
            .count()
    }
}
