# Observer

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/observer/rust/example](https://refactoring.guru/design-patterns/observer/rust/example)

## Intent
Implement the Observer pattern in Rust using function pointers and HashMap to decouple event publishers from subscribers.

## Rust Idiom
Rust uses function pointers (`fn(T)`) and `HashMap<K, Vec<Subscriber>>` instead of trait objects to store heterogeneous callbacks. This avoids the vtable indirection of `dyn Observer` while maintaining type safety. The pattern leverages Rust's zero-cost abstractions: storing bare function pointers means no heap allocation per listener, and cloning `Event` (a small enum) is cheaper than vtable dispatch. For dynamic dispatch scenarios (e.g., closures capturing state), `Box<dyn Fn(T)>` replaces `fn(T)`, trading zero-cost for flexibility.

## Key Participants
- Publisher: Manages event-to-subscribers mapping via HashMap<Event, Vec<Subscriber>>. Implements subscribe(), unsubscribe(), and notify().
- Subscriber: Type alias fn(String) -> () representing a stateless callback.
- Event: Enum discriminant keyed in the HashMap (must derive Hash, Eq, Clone).
- Editor: Concrete subject embedding a Publisher and calling notify() on state changes.

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Event {
    Load,
    Save,
}

pub type Subscriber = fn(String);

#[derive(Default)]
pub struct Publisher {
    events: HashMap<Event, Vec<Subscriber>>,
}

impl Publisher {
    pub fn subscribe(&mut self, event: Event, listener: Subscriber) {
        self.events.entry(event).or_default().push(listener);
    }

    pub fn unsubscribe(&mut self, event: Event, listener: Subscriber) {
        if let Some(listeners) = self.events.get_mut(&event) {
            listeners.retain(|&x| x != listener);
        }
    }

    pub fn notify(&self, event: Event, data: String) {
        if let Some(listeners) = self.events.get(&event) {
            for listener in listeners {
                listener(data.clone());
            }
        }
    }
}

pub struct Editor {
    publisher: Publisher,
    file_path: String,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            publisher: Publisher::default(),
            file_path: String::new(),
        }
    }

    pub fn events(&mut self) -> &mut Publisher {
        &mut self.publisher
    }

    pub fn load(&mut self, path: String) {
        self.file_path = path.clone();
        self.publisher.notify(Event::Load, path);
    }

    pub fn save(&self) {
        self.publisher.notify(Event::Save, self.file_path.clone());
    }
}

fn log_open(file: String) {
    println!("[LOG] Opened file: {}", file);
}

fn log_save(file: String) {
    println!("[LOG] Saved file: {}", file);
}

fn main() {
    let mut editor = Editor::new();
    editor.events().subscribe(Event::Load, log_open);
    editor.events().subscribe(Event::Save, log_save);

    editor.load("document.txt".to_string());
    editor.save();
}
```

## When to Use
- Event systems where subscribers are stateless functions or can be represented as function pointers (GUIs, logging, metrics).
- Decoupling a Subject (Publisher/Editor) from its observers, enabling independent change without recompilation.
- Real-time reactive updates: file editors, form validators, configuration watchers.
- Avoiding vtable overhead when callbacks are simple and do not capture external state.

## Rust Caveats (ownership / borrow / dispatch)
- Function pointer equality: fn(T) implements Eq, so retain(|&x| x != listener) works for unsubscribe. Closures with captured state cannot be stored as bare fn — switch to Box<dyn Fn(T)> if state capture is needed, accept the heap allocation and vtable cost.
- Clone penalty on notify: Every listener call clones data.clone() in the loop. For large strings, pre-allocate or refactor to pass &str; HashMap does not enforce the Subscriber signature, so changing it breaks all call sites.
- No removal by identity in dyn closures: If you use Box<dyn Fn(T)>, function pointer equality is gone — you must track IDs (e.g., HashMap<u64, Box<dyn Fn(T)>>) or provide an explicit unsubscribe_id() method.
- Lifetime entanglement if subscribers reference the subject: Storing &'a Editor in a closure and then subscribing that closure to editor.events() creates a borrow cycle. Use interior mutability (Cell/RefCell) or restructure to decouple lifetimes.
- Cloning enums as keys: Every notify() clones Event to look it up in the HashMap. For large event trees, intern events as &'static str or numeric tags to avoid allocations.
