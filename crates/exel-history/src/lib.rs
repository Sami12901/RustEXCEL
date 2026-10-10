use std::collections::VecDeque;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum HistoryError {
    #[error("No commands available to undo")]
    NothingToUndo,
    #[error("No commands available to redo")]
    NothingToRedo,
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Transaction error: {0}")]
    TransactionError(String),
}

/// A generic undoable command that mutates state `T`.
pub trait HistoryCommand<T>: Debug + Send + Sync {
    /// Execute the forward mutation on `target`.
    fn execute(&mut self, target: &mut T) -> Result<(), HistoryError>;

    /// Reverse the mutation on `target`, restoring prior state.
    fn undo(&mut self, target: &mut T) -> Result<(), HistoryError>;

    /// Human-readable label for UI undo/redo menus (e.g. "Typing in A1", "Delete Row 4").
    fn description(&self) -> &str;
}

/// A compound batch of commands treated as a single atomic undo/redo unit.
pub struct CompoundCommand<T> {
    pub description: String,
    pub commands: Vec<Box<dyn HistoryCommand<T>>>,
}

impl<T> Debug for CompoundCommand<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompoundCommand")
            .field("description", &self.description)
            .field("commands_count", &self.commands.len())
            .finish()
    }
}

impl<T> CompoundCommand<T> {
    pub fn new<S: Into<String>>(description: S, commands: Vec<Box<dyn HistoryCommand<T>>>) -> Self {
        Self {
            description: description.into(),
            commands,
        }
    }
}

impl<T> HistoryCommand<T> for CompoundCommand<T> {
    fn execute(&mut self, target: &mut T) -> Result<(), HistoryError> {
        for cmd in &mut self.commands {
            cmd.execute(target)?;
        }
        Ok(())
    }

    fn undo(&mut self, target: &mut T) -> Result<(), HistoryError> {
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(target)?;
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Bounded undo/redo history manager.
pub struct HistoryManager<T> {
    undo_stack: VecDeque<Box<dyn HistoryCommand<T>>>,
    redo_stack: Vec<Box<dyn HistoryCommand<T>>>,
    max_depth: usize,
    active_transaction: Option<TransactionBatch<T>>,
    clean_version: usize,
    current_version: usize,
}

struct TransactionBatch<T> {
    description: String,
    commands: Vec<Box<dyn HistoryCommand<T>>>,
}

impl<T: 'static> Default for HistoryManager<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

impl<T: 'static> HistoryManager<T> {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_depth.min(1000)),
            redo_stack: Vec::new(),
            max_depth: max_depth.max(1),
            active_transaction: None,
            clean_version: 0,
            current_version: 0,
        }
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth.max(1);
        while self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }

    /// Executes the command on `target` and records it onto the undo stack.
    /// Clears the redo stack.
    pub fn execute(
        &mut self,
        mut command: Box<dyn HistoryCommand<T>>,
        target: &mut T,
    ) -> Result<(), HistoryError> {
        command.execute(target)?;

        if let Some(tx) = &mut self.active_transaction {
            tx.commands.push(command);
        } else {
            self.redo_stack.clear();
            if self.undo_stack.len() >= self.max_depth {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(command);
            self.current_version = self.current_version.wrapping_add(1);
        }

        Ok(())
    }

    /// Records an already-executed command onto the undo stack.
    pub fn record(&mut self, command: Box<dyn HistoryCommand<T>>) {
        if let Some(tx) = &mut self.active_transaction {
            tx.commands.push(command);
        } else {
            self.redo_stack.clear();
            if self.undo_stack.len() >= self.max_depth {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(command);
            self.current_version = self.current_version.wrapping_add(1);
        }
    }

    /// Undoes the most recent command, returning its description.
    pub fn undo(&mut self, target: &mut T) -> Result<String, HistoryError> {
        if self.active_transaction.is_some() {
            return Err(HistoryError::TransactionError(
                "Cannot undo while a transaction is actively open".to_string(),
            ));
        }

        let mut cmd = self
            .undo_stack
            .pop_back()
            .ok_or(HistoryError::NothingToUndo)?;
        if let Err(e) = cmd.undo(target) {
            // Re-insert command so stack remains valid
            self.undo_stack.push_back(cmd);
            return Err(e);
        }

        let desc = cmd.description().to_string();
        self.redo_stack.push(cmd);
        self.current_version = self.current_version.wrapping_sub(1);
        Ok(desc)
    }

    /// Redoes the most recently undone command, returning its description.
    pub fn redo(&mut self, target: &mut T) -> Result<String, HistoryError> {
        if self.active_transaction.is_some() {
            return Err(HistoryError::TransactionError(
                "Cannot redo while a transaction is actively open".to_string(),
            ));
        }

        let mut cmd = self.redo_stack.pop().ok_or(HistoryError::NothingToRedo)?;
        if let Err(e) = cmd.execute(target) {
            self.redo_stack.push(cmd);
            return Err(e);
        }

        let desc = cmd.description().to_string();
        self.undo_stack.push_back(cmd);
        self.current_version = self.current_version.wrapping_add(1);
        Ok(desc)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() && self.active_transaction.is_none()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty() && self.active_transaction.is_none()
    }

    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|cmd| cmd.description())
    }

    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|cmd| cmd.description())
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.active_transaction = None;
        self.clean_version = 0;
        self.current_version = 0;
    }

    /// Mark current state as saved / clean.
    pub fn mark_clean(&mut self) {
        self.clean_version = self.current_version;
    }

    /// Check if target state has unsaved mutations compared to mark_clean point.
    pub fn is_dirty(&self) -> bool {
        self.clean_version != self.current_version
    }

    /// Start a transactional grouping of multiple commands.
    pub fn begin_transaction<S: Into<String>>(
        &mut self,
        description: S,
    ) -> Result<(), HistoryError> {
        if self.active_transaction.is_some() {
            return Err(HistoryError::TransactionError(
                "Transaction already in progress".to_string(),
            ));
        }
        self.active_transaction = Some(TransactionBatch {
            description: description.into(),
            commands: Vec::new(),
        });
        Ok(())
    }

    /// Commit the current transaction into a single compound command on the undo stack.
    pub fn commit_transaction(&mut self) -> Result<(), HistoryError> {
        let tx = self
            .active_transaction
            .take()
            .ok_or_else(|| HistoryError::TransactionError("No active transaction".to_string()))?;

        if !tx.commands.is_empty() {
            let compound = CompoundCommand::new(tx.description, tx.commands);
            self.redo_stack.clear();
            if self.undo_stack.len() >= self.max_depth {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(Box::new(compound));
            self.current_version = self.current_version.wrapping_add(1);
        }
        Ok(())
    }

    /// Abort the current transaction and undo all mutations that occurred within it.
    pub fn rollback_transaction(&mut self, target: &mut T) -> Result<(), HistoryError> {
        let mut tx = self
            .active_transaction
            .take()
            .ok_or_else(|| HistoryError::TransactionError("No active transaction".to_string()))?;

        for cmd in tx.commands.iter_mut().rev() {
            cmd.undo(target)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    struct Counter {
        val: i32,
    }

    #[derive(Debug)]
    struct AddCommand {
        delta: i32,
    }

    impl HistoryCommand<Counter> for AddCommand {
        fn execute(&mut self, target: &mut Counter) -> Result<(), HistoryError> {
            target.val += self.delta;
            Ok(())
        }

        fn undo(&mut self, target: &mut Counter) -> Result<(), HistoryError> {
            target.val -= self.delta;
            Ok(())
        }

        fn description(&self) -> &str {
            "Add delta"
        }
    }

    #[test]
    fn test_undo_redo_stack() {
        let mut state = Counter { val: 0 };
        let mut history = HistoryManager::new(5);

        assert!(!history.can_undo());
        assert!(!history.can_redo());

        history
            .execute(Box::new(AddCommand { delta: 10 }), &mut state)
            .unwrap();
        assert_eq!(state.val, 10);
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert!(history.is_dirty());

        history.mark_clean();
        assert!(!history.is_dirty());

        history
            .execute(Box::new(AddCommand { delta: 5 }), &mut state)
            .unwrap();
        assert_eq!(state.val, 15);
        assert!(history.is_dirty());

        // Undo
        let desc = history.undo(&mut state).unwrap();
        assert_eq!(desc, "Add delta");
        assert_eq!(state.val, 10);
        assert!(!history.is_dirty()); // returned to clean point!
        assert!(history.can_redo());

        // Redo
        history.redo(&mut state).unwrap();
        assert_eq!(state.val, 15);
        assert!(history.is_dirty());
    }

    #[test]
    fn test_bounded_stack() {
        let mut state = Counter { val: 0 };
        let mut history = HistoryManager::new(3);

        for i in 1..=5 {
            history
                .execute(Box::new(AddCommand { delta: i }), &mut state)
                .unwrap();
        }

        assert_eq!(history.undo_count(), 3);
        assert_eq!(state.val, 15);

        // Can only undo 3 times
        history.undo(&mut state).unwrap();
        history.undo(&mut state).unwrap();
        history.undo(&mut state).unwrap();
        assert!(!history.can_undo());
    }

    #[test]
    fn test_transaction_commit_and_rollback() {
        let mut state = Counter { val: 100 };
        let mut history = HistoryManager::new(10);

        history.begin_transaction("Batch changes").unwrap();
        history
            .execute(Box::new(AddCommand { delta: 10 }), &mut state)
            .unwrap();
        history
            .execute(Box::new(AddCommand { delta: 20 }), &mut state)
            .unwrap();
        assert_eq!(state.val, 130);
        history.commit_transaction().unwrap();

        assert_eq!(history.undo_count(), 1);
        history.undo(&mut state).unwrap();
        assert_eq!(state.val, 100);

        // Rollback test
        history.begin_transaction("Failing batch").unwrap();
        history
            .execute(Box::new(AddCommand { delta: 50 }), &mut state)
            .unwrap();
        assert_eq!(state.val, 150);
        history.rollback_transaction(&mut state).unwrap();
        assert_eq!(state.val, 100);
        assert_eq!(history.undo_count(), 0); // No new command added
    }
}
