# State

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/state/rust/example](https://refactoring.guru/design-patterns/state/rust/example)

## Intent
Encapsulate state-dependent behavior into objects that implement a common trait, allowing state transitions without conditionals.

## Rust Idiom
Rust implements State via trait objects (`Box<dyn State>`). Methods take `self: Box<Self>` to consume and replace the state, leveraging move semantics and ownership to ensure type-safe transitions. The context holds a boxed trait object; mutations via method calls replace the entire state atomically. Generics-based alternatives use associated types or enum-wrapped concrete states for compile-time dispatch.

## Key Participants
- trait State: defines play/stop/pause methods taking Box<Self>
- struct Playing, Paused, Stopped: concrete state implementations
- struct Player: context holding Box<dyn State>
- Box<dyn State>: type-erased state enabling runtime polymorphism

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::fmt;

// State trait: defines behavior for each state
trait State: fmt::Debug {
    fn play(self: Box<Self>) -> Box<dyn State>;
    fn stop(self: Box<Self>) -> Box<dyn State>;
}

// Concrete states
#[derive(Debug)]
struct Playing;

#[derive(Debug)]
struct Paused;

#[derive(Debug)]
struct Stopped;

impl State for Playing {
    fn play(self: Box<Self>) -> Box<dyn State> {
        println!("Already playing");
        self
    }
    fn stop(self: Box<Self>) -> Box<dyn State> {
        println!("Stopping playback");
        Box::new(Stopped)
    }
}

impl State for Paused {
    fn play(self: Box<Self>) -> Box<dyn State> {
        println!("Resuming playback");
        Box::new(Playing)
    }
    fn stop(self: Box<Self>) -> Box<dyn State> {
        println!("Stopping from pause");
        Box::new(Stopped)
    }
}

impl State for Stopped {
    fn play(self: Box<Self>) -> Box<dyn State> {
        println!("Starting playback");
        Box::new(Playing)
    }
    fn stop(self: Box<Self>) -> Box<dyn State> {
        println!("Already stopped");
        self
    }
}

// Context: holds current state.
// `Box<dyn State>` has no `Default`, so `mem::take` cannot lift the state out.
// `Option<Box<dyn State>>` + `Option::take()` is the idiomatic move: take leaves
// `None` behind, we consume the owned state via `self: Box<Self>`, then write the
// successor back. The `None` window is invisible because `&mut self` is exclusive.
struct Player {
    state: Option<Box<dyn State>>,
}

impl Player {
    fn new() -> Self {
        Player {
            state: Some(Box::new(Stopped)),
        }
    }

    fn play(&mut self) {
        let current = self.state.take().expect("state is always Some between calls");
        self.state = Some(current.play());
    }

    fn stop(&mut self) {
        let current = self.state.take().expect("state is always Some between calls");
        self.state = Some(current.stop());
    }
}

fn main() {
    let mut player = Player::new();
    player.play();   // Starting playback
    player.play();   // Already playing
    player.stop();   // Stopping playback
    player.stop();   // Already stopped
}
```

## When to Use
- State machines with multiple distinct behaviors per state (e.g., protocol handlers, UI workflow engines)
- Avoiding large match/if chains that grow with each new state
- Runtime state transitions where the next state type may vary based on input or side effects
- Encapsulating state-specific logic so each state owns its own transitions

## Rust Caveats (ownership / borrow / dispatch)
- `std::mem::take` needs `Default`, which `Box<dyn State>` does NOT implement (E0277) — the naive `self.state = mem::take(&mut self.state).play()` will not compile. Model the slot as `Option<Box<dyn State>>` and use `Option::take()`, which leaves `None` behind without requiring `Default`.
- `self: Box<Self>` consumption forces total replacement: a transition consumes the old state and returns the next, so a method cannot mutate the current state in place or perform a partial/invalid transition — the type system makes illegal intermediate states unrepresentable.
- Trait objects erase type info: every state impls the same `State` trait with identical signatures; the context cannot read state-specific fields without `downcast` (verbose) — push per-state data behind the trait methods instead.
- No reference survives a transition: the context takes ownership of the old `Box` and drops it, so you cannot hold a borrow into the prior state across the transition; thread any carry-over data through the method return type.
- Allocation per transition: each move boxes a new state. On hot paths replace `Box<dyn State>` with an `enum` of states + a `match`, compiling the machine to concrete types and removing both the heap churn and the vtable dispatch.
