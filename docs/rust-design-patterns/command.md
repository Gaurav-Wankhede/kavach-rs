# Command

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/command/rust/example](https://refactoring.guru/design-patterns/command/rust/example)

## Intent
Encapsulate a request as an object, parameterize invokers with commands, queue/undo, and decouple senders from receivers.

## Rust Idiom
Rust realizes Command via trait objects (dyn Command) stored in a Vec for undo history. Commands are stateful structs implementing a common trait with execute() and undo() methods. The invoker (dispatcher) holds mutable references to shared state and passes them to commands at runtime, avoiding the receiver holding permanent references. Ownership is strict: backup state lives in command instances (via owned String), state mutations happen through &mut parameters, and the trait object Vec manages lifetime.

## Key Participants
- trait Command — encapsulates behavior (execute, undo)
- struct CopyCommand, CutCommand, PasteCommand — concrete commands holding their own backups
- struct Invoker — holds command history (Vec<Box<dyn Command>>) and executes/undoes them
- struct AppState — the mutable state shared across commands (text content, clipboard)
- dyn Command — runtime polymorphism via trait object for heterogeneous command queue

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::collections::VecDeque;

// The Command trait — every command must be executable and undoable.
trait Command {
    fn execute(&mut self, state: &mut AppState) -> bool; // returns true if undoable
    fn undo(&mut self, state: &mut AppState);
}

// Shared mutable state that all commands access.
#[derive(Clone)]
struct AppState {
    text: String,
    clipboard: String,
}

// Copy command — read-only, no backup needed.
struct CopyCommand;
impl Command for CopyCommand {
    fn execute(&mut self, state: &mut AppState) -> bool {
        state.clipboard = state.text.clone();
        false // not undoable
    }
    fn undo(&mut self, _state: &mut AppState) {}
}

// Cut command — mutates state, backup required.
struct CutCommand {
    backup: String,
}
impl Command for CutCommand {
    fn execute(&mut self, state: &mut AppState) -> bool {
        self.backup = state.text.clone();
        state.clipboard = self.backup.clone();
        state.text.clear();
        true // undoable
    }
    fn undo(&mut self, state: &mut AppState) {
        state.text = self.backup.clone();
    }
}

// Paste command — mutates state, backup required.
struct PasteCommand {
    backup: String,
}
impl Command for PasteCommand {
    fn execute(&mut self, state: &mut AppState) -> bool {
        self.backup = state.text.clone();
        state.text = state.clipboard.clone();
        true // undoable
    }
    fn undo(&mut self, state: &mut AppState) {
        state.text = self.backup.clone();
    }
}

// The Invoker — manages command history and execution.
struct Invoker {
    history: VecDeque<Box<dyn Command>>,
}
impl Invoker {
    fn new() -> Self {
        Invoker { history: VecDeque::new() }
    }
    fn execute(&mut self, mut cmd: Box<dyn Command>, state: &mut AppState) {
        if cmd.execute(state) {
            self.history.push_back(cmd);
        }
    }
    fn undo(&mut self, state: &mut AppState) {
        if let Some(mut cmd) = self.history.pop_back() {
            cmd.undo(state);
        }
    }
}

fn main() {
    let mut state = AppState { text: "Hello".into(), clipboard: String::new() };
    let mut invoker = Invoker::new();

    println!("Initial: text='{}', clipboard='{}'", state.text, state.clipboard);

    invoker.execute(Box::new(CopyCommand), &mut state);
    println!("After Copy: text='{}', clipboard='{}'", state.text, state.clipboard);

    invoker.execute(Box::new(CutCommand { backup: String::new() }), &mut state);
    println!("After Cut: text='{}', clipboard='{}'", state.text, state.clipboard);

    invoker.execute(Box::new(PasteCommand { backup: String::new() }), &mut state);
    println!("After Paste: text='{}', clipboard='{}'", state.text, state.clipboard);

    invoker.undo(&mut state);
    println!("After Undo: text='{}', clipboard='{}'", state.text, state.clipboard);
}
```

## When to Use
- Task queuing systems where commands are enqueued and executed later (job schedulers, event handlers)
- Undo/redo stacks in editors or interactive applications — backup state stored in command instances
- Deferred execution and parameterization of requests (e.g., scheduling, async task dispatch)
- Decoupling invokers from receivers so commands can be swapped, logged, or serialized independently

## Rust Caveats (ownership / borrow / dispatch)
- Trait objects (dyn Command) erase type information — you lose compile-time dispatch and must pay vtable cost; use generics with execute::<T: Command>() if monomorphism is an option
- Commands holding backup state as owned String means cloning on every undo — consider Cow<str> or Rc<RefCell<String>> for large payloads, but then borrow rules become complex during undo
- The history Vec<Box<dyn Command>> pins commands to the heap; popping and calling undo() requires moving the command out, which is fine only if undo() doesn't need other history entries — avoid mutual references in the history
- State mutations happen through &mut AppState passed at runtime, not held by the command; if state is behind a Mutex/Arc for multi-threaded use, command.execute() must lock/unlock, adding contention — design the state shape first
- Commands implementing undo() must preserve enough backup to restore the exact prior state; if AppState contains references or Rc/Arc, backup becomes non-trivial (deep clone vs. reference counting) — keep commands stateless where possible or use explicit backup fields
