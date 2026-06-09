# Adapter

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/adapter/rust/example](https://refactoring.guru/design-patterns/adapter/rust/example)

## Intent
Wrap an incompatible interface inside a compatible trait to enable collaboration without modifying the adaptee.

## Rust Idiom
Rust adapts via trait impls over composition. The adapter wraps the incompatible type (adaptee) and implements the target trait, delegating to adaptee's methods and transforming the result. No subclassing—trait objects (dyn) or generics handle polymorphism. Private fields enforce encapsulation.

## Key Participants
- Target (trait): the expected interface clients depend on
- TargetAdapter (struct): wraps the adaptee and implements Target
- SpecificTarget (struct): the incompatible type needing adaptation
- OrdinaryTarget (struct): a direct impl of Target, showing the contrast

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::fmt;

// Target: the trait clients expect
pub trait Target {
    fn request(&self) -> String;
}

// Adaptee: the incompatible type we need to wrap
pub struct SpecificTarget;

impl SpecificTarget {
    pub fn specific_request(&self) -> String {
        ".tseuqer cificepS".into()
    }
}

// Adapter: wraps adaptee and implements Target
pub struct TargetAdapter {
    adaptee: SpecificTarget,
}

impl TargetAdapter {
    pub fn new(adaptee: SpecificTarget) -> Self {
        Self { adaptee }
    }
}

impl Target for TargetAdapter {
    fn request(&self) -> String {
        // Transform adaptee's interface to match Target
        self.adaptee.specific_request().chars().rev().collect()
    }
}

// A compatible target (shows the contrast)
pub struct OrdinaryTarget;

impl Target for OrdinaryTarget {
    fn request(&self) -> String {
        "Ordinary request.".into()
    }
}

// Client code: polymorphic via trait object or generic
fn call<T: Target>(target: &T) {
    println!("Response: '{}'", target.request());
}

fn main() {
    let ordinary = OrdinaryTarget;
    print!("Compatible target: ");
    call(&ordinary);

    let specific = SpecificTarget;
    println!("Incompatible adaptee: '{}'", specific.specific_request());

    let adapter = TargetAdapter::new(specific);
    print!("Adapted target: ");
    call(&adapter);
}
```

## When to Use
- Integrating a third-party library with an incompatible API into your trait-based architecture
- Converting between two incompatible trait interfaces when you cannot modify either
- Bridging legacy code (returns raw data) with modern generic code expecting a specific trait
- Making an owned type temporarily satisfy a borrowed trait (wrap in adapter that takes &self and borrows the owned field)

## Rust Caveats (ownership / borrow / dispatch)
- Borrowing in delegation: if adaptee is borrowed (&self), the adapter's impl must also borrow—mutable delegation is blocked unless adaptee is interior-mutable (Cell/RefCell)
- Lifetime coupling: adapter's lifetime is tied to adaptee's lifetime if adaptee is borrowed; move semantics (owned adaptee) sidestep this but use more memory
- dyn Trait overhead: using dyn Target for polymorphism adds vtable indirection; prefer generic <T: Target> where possible for monomorphization and zero-cost abstraction
- Self-referential adapters forbidden: if adapter needs to hold &self references to adaptee fields, Rust's rules prevent self-referential structs without Pin or unsafe; composition with owned adaptee avoids this
