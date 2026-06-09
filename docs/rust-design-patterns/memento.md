# Memento

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/memento/rust/example](https://refactoring.guru/design-patterns/memento/rust/example)

## Intent
Capture object state without exposing internal representation, enabling undo/restore without breaking encapsulation.

## Rust Idiom
Rust implements Memento through owned type-safe snapshots: the Originator creates an immutable Memento struct (typically a private or sealed variant), which the Caretaker holds in a Vec. Unlike OOP's opaque mementos, Rust's ownership model makes the snapshot self-sealing—the memento type cannot be constructed or mutated externally. Generics or trait objects let the Caretaker remain generic over memento types; associated types bind the memento to its originator. Serde offers a serialization-based variant (String-backed JSON), trading type safety for flexibility.

## Key Participants
- Originator: holds mutable state, produces immutable Memento snapshots via save()
- Memento (OriginatorMemento): an immutable struct capturing state, implements Memento trait to restore itself
- Caretaker (history Vec): holds ordered Mementos, orchestrates undo/redo by pushing/popping
- Memento trait: defines restore() which consumes self and returns Originator

## Reference Implementation (Rust 2024, compiles standalone)
```rust

// Type-safe Memento: snapshot sealed by ownership & type system.
trait Memento {
    fn restore(self) -> Originator;
}

struct Originator {
    state: u32,
}

impl Originator {
    fn new(state: u32) -> Self {
        Originator { state }
    }

    fn set_state(&mut self, state: u32) {
        self.state = state;
    }

    fn save(&self) -> OriginatorMemento {
        OriginatorMemento {
            state: self.state,
        }
    }
}

struct OriginatorMemento {
    state: u32,
}

impl Memento for OriginatorMemento {
    fn restore(self) -> Originator {
        Originator { state: self.state }
    }
}

fn main() {
    let mut orig = Originator::new(0);
    let mut history: Vec<OriginatorMemento> = Vec::new();

    orig.set_state(1);
    history.push(orig.save());

    orig.set_state(2);
    history.push(orig.save());

    orig.set_state(3);
    history.push(orig.save());

    println!("Current state: {}", orig.state);

    if let Some(memento) = history.pop() {
        orig = memento.restore();
        println!("Restored to: {}", orig.state);
    }

    if let Some(memento) = history.pop() {
        orig = memento.restore();
        println!("Restored to: {}", orig.state);
    }
}
```

## When to Use
- Undo/redo stacks in editors: capture editor state, restore on Ctrl+Z
- Game save/load: freeze game state to memento, reload from stored snapshot
- Transaction rollback: snapshot transaction state before apply, restore on abort
- Configuration versioning: preserve config snapshots, revert to prior version without re-reading source

## Rust Caveats (ownership / borrow / dispatch)
- Ownership transfer on restore(): the memento consumes itself (self) to ensure it is not reused after restoration; attempting to save and restore the same snapshot twice requires cloning or re-creating the memento
- Generic lifetime of Memento trait: if the Originator holds borrowed data, the Memento must also hold owned copies (e.g., String instead of &str) or declare lifetimes on the trait, complicating the Caretaker's Vec<Box<dyn Memento>> signature
- Type erasure cost: using trait objects (dyn Memento) requires Box and dynamic dispatch; prefer generic Caretaker<M: Memento> if all mementos share one type, or a sealed enum if there are known variants
- Serde variant (String-based): parse errors are runtime (unwrap-prone); the type information is lost, so restoring the wrong state type silently parses as zero; always validate or use strongly-typed serialization
