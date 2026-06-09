# Bridge

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/bridge/rust/example](https://refactoring.guru/design-patterns/bridge/rust/example)

## Intent
Decouple an abstraction from its implementation so the two can vary independently, using trait composition and generics.

## Rust Idiom
Bridge in Rust uses **generic traits** (not inheritance) to separate abstraction (Remote) from implementation (Device). The abstraction holds a generic type parameter `D: Device`, and trait bounds enforce the implementation contract. Mutable access flows through a helper trait (`HasMutableDevice`) to satisfy borrowing rules—the key insight is that trait default methods can call `&mut self` on the implementation via the accessor, avoiding self-referential borrow issues that would arise in languages with nominal inheritance.

## Key Participants
- Device trait: the implementation abstraction (TV, Radio implement it)
- Remote<D: Device> trait: the abstraction layer with default methods
- HasMutableDevice<D: Device> trait: accessor for mutable device reference, required because trait methods need &mut self to call device operations
- BasicRemote<D> and AdvancedRemote<D>: concrete abstractions, each holding a device: D
- Tv, Radio: concrete implementations of Device

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::cmp;

// Implementation abstraction: device interface
trait Device {
    fn is_enabled(&self) -> bool;
    fn enable(&mut self);
    fn disable(&mut self);
    fn volume(&self) -> u8;
    fn set_volume(&mut self, percent: u8);
}

// Concrete implementations
#[derive(Clone)]
struct Tv { on: bool, volume: u8 }

impl Default for Tv {
    fn default() -> Self {
        Self { on: false, volume: 30 }
    }
}

impl Device for Tv {
    fn is_enabled(&self) -> bool { self.on }
    fn enable(&mut self) { self.on = true; }
    fn disable(&mut self) { self.on = false; }
    fn volume(&self) -> u8 { self.volume }
    fn set_volume(&mut self, percent: u8) {
        self.volume = cmp::min(percent, 100);
    }
}

#[derive(Clone)]
struct Radio { on: bool, volume: u8 }

impl Default for Radio {
    fn default() -> Self {
        Self { on: false, volume: 30 }
    }
}

impl Device for Radio {
    fn is_enabled(&self) -> bool { self.on }
    fn enable(&mut self) { self.on = true; }
    fn disable(&mut self) { self.on = false; }
    fn volume(&self) -> u8 { self.volume }
    fn set_volume(&mut self, percent: u8) {
        self.volume = cmp::min(percent, 100);
    }
}

// Accessor trait: allows Remote methods to get &mut D
trait HasMutableDevice<D: Device> {
    fn device(&mut self) -> &mut D;
}

// Abstraction: remote control (works with any Device)
trait Remote<D: Device>: HasMutableDevice<D> {
    fn power(&mut self) {
        if self.device().is_enabled() {
            self.device().disable();
        } else {
            self.device().enable();
        }
    }

    fn volume_up(&mut self) {
        let vol = self.device().volume();
        self.device().set_volume(vol + 10);
    }

    fn volume_down(&mut self) {
        let vol = self.device().volume();
        self.device().set_volume(vol.saturating_sub(10));
    }
}

// Basic concrete abstraction
struct BasicRemote<D: Device> {
    device: D,
}

impl<D: Device> BasicRemote<D> {
    fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D: Device> HasMutableDevice<D> for BasicRemote<D> {
    fn device(&mut self) -> &mut D {
        &mut self.device
    }
}

impl<D: Device> Remote<D> for BasicRemote<D> {}

// Advanced concrete abstraction
struct AdvancedRemote<D: Device> {
    device: D,
}

impl<D: Device> AdvancedRemote<D> {
    fn new(device: D) -> Self {
        Self { device }
    }

    fn mute(&mut self) {
        self.device.set_volume(0);
    }
}

impl<D: Device> HasMutableDevice<D> for AdvancedRemote<D> {
    fn device(&mut self) -> &mut D {
        &mut self.device
    }
}

impl<D: Device> Remote<D> for AdvancedRemote<D> {}

fn main() {
    let mut tv_remote = BasicRemote::new(Tv::default());
    tv_remote.power();
    println!("TV enabled: {}", tv_remote.device().is_enabled());
    tv_remote.volume_up();
    println!("TV volume: {}", tv_remote.device().volume());

    let mut radio_remote = AdvancedRemote::new(Radio::default());
    radio_remote.power();
    radio_remote.volume_up();
    radio_remote.mute();
    println!("Radio volume after mute: {}", radio_remote.device().volume());
}
```

## When to Use
- You have two parallel hierarchies (abstractions like Remote and implementations like Device) that need to vary independently
- You want to avoid rigid class hierarchies (e.g., TvRemote, RadioRemote, BasicTvRemote, AdvancedRadioRemote matrix)
- You need to defer implementation choice at runtime while keeping the abstraction interface stable
- A new Device type should not require new Remote types, and vice versa

## Rust Caveats (ownership / borrow / dispatch)
- Mutable access bottleneck: trait methods that need &mut D require HasMutableDevice<D> as a supertrait because self in a trait method is already borrowed. Use the accessor pattern to get mutable access to the implementation.
- Generic monomorphization: each Remote<D> instantiation with a different D generates its own code. The trade-off is code duplication vs. static dispatch; use dyn Device if you need true runtime polymorphism (but then lose zero-cost abstraction).
- Lifetime and ownership: the Remote owns the Device (holds it as a field). If Device must be borrowed from elsewhere, add lifetime parameters: Remote<'a, D> with device: &'a mut D. This complicates trait bounds but enables external ownership.
- No self-referential methods: a trait method cannot return a reference to both self and self.device() in the same signature—the borrow checker forbids it. Default methods can call device operations but cannot return a reference bridging the two scopes.
