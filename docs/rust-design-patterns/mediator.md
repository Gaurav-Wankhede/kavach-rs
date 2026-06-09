# Mediator

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/mediator/rust/example](https://refactoring.guru/design-patterns/mediator/rust/example)

## Intent
Centralize object interactions through a mediator to reduce coupling between components and decouple business logic from coordination.

## Rust Idiom
Rust uses trait objects (`dyn Mediator`) to represent the mediator abstraction, allowing concrete implementations to manage shared state and orchestrate interactions. Components interact only with the mediator via method parameters (not retained references), leveraging Rust's ownership model to prevent circular dependencies. Concrete mediators (e.g., `TrainStation`) own all participants in a `HashMap<String, Box<dyn Train>>`, moving ownership in and out as needed, eliminating the reference-holding complexity of OOP mediators. Enums can replace multiple boolean state variables (e.g., `Option<String>` for platform occupancy).

## Key Participants
- Mediator trait: defines the interface for coordinating colleagues (arrival/departure notifications)
- Train trait: defines the interface for colleagues (name, arrive, depart methods)
- TrainStation struct: concrete mediator that owns all trains and manages platform scheduling
- PassengerTrain struct: concrete colleague implementing Train

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::collections::{HashMap, VecDeque};

// Mediator trait: coordinates interactions between colleagues
trait Mediator {
    fn notify_arrival(&mut self, name: &str) -> bool;
    fn notify_departure(&mut self, name: &str);
}

// Colleague trait
trait Train {
    fn name(&self) -> &str;
    fn arrive(&mut self, mediator: &mut dyn Mediator);
    fn depart(&mut self, mediator: &mut dyn Mediator);
}

// Concrete colleague
struct PassengerTrain {
    name: String,
}

impl PassengerTrain {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl Train for PassengerTrain {
    fn name(&self) -> &str {
        &self.name
    }

    fn arrive(&mut self, mediator: &mut dyn Mediator) {
        if mediator.notify_arrival(self.name()) {
            println!("Passenger train {}: Arrived on platform", self.name);
        } else {
            println!("Passenger train {}: Waiting in queue", self.name);
        }
    }

    fn depart(&mut self, mediator: &mut dyn Mediator) {
        println!("Passenger train {}: Departing", self.name);
        mediator.notify_departure(self.name());
    }
}

// Concrete mediator: owns all colleagues
struct TrainStation {
    trains: HashMap<String, Box<dyn Train>>,
    queue: VecDeque<String>,
    platform_occupant: Option<String>,
}

impl TrainStation {
    fn new() -> Self {
        Self {
            trains: HashMap::new(),
            queue: VecDeque::new(),
            platform_occupant: None,
        }
    }

    fn accept(&mut self, mut train: impl Train + 'static) {
        train.arrive(self);
        self.trains.insert(train.name().to_string(), Box::new(train));
    }

    fn depart(&mut self, name: &str) {
        if let Some(mut train) = self.trains.remove(name) {
            train.depart(self);
        }
    }
}

impl Mediator for TrainStation {
    fn notify_arrival(&mut self, name: &str) -> bool {
        if self.platform_occupant.is_some() {
            self.queue.push_back(name.to_string());
            false
        } else {
            self.platform_occupant = Some(name.to_string());
            true
        }
    }

    fn notify_departure(&mut self, _name: &str) {
        self.platform_occupant = None;
        if let Some(next_name) = self.queue.pop_front() {
            if let Some(mut next_train) = self.trains.remove(&next_name) {
                next_train.arrive(self);
                self.trains.insert(next_name.clone(), next_train);
                self.platform_occupant = Some(next_name);
            }
        }
    }
}

fn main() {
    let mut station = TrainStation::new();

    let p1 = PassengerTrain::new("P1");
    let p2 = PassengerTrain::new("P2");

    station.accept(p1);
    station.accept(p2);

    station.depart("P1");
    station.depart("P2");
}
```

## When to Use
- Dialog boxes with many interdependent controls (mediator coordinates validation and state propagation)
- Game event systems where many entities react to each other (mediator broadcasts events instead of direct coupling)
- Chat room implementations where users send messages through a mediator rather than directly to each other
- Workflow orchestrators managing multiple sequential or conditional steps (mediator routes control flow)

## Rust Caveats (ownership / borrow / dispatch)
- Ownership pinning: the mediator must own colleagues (as `Box<dyn Trait>`) to avoid self-referential lifetimes; colleagues cannot hold references back without creating cycles
- dyn Mediator parameter passing: colleagues accept mediator as `&mut dyn Mediator` (borrowed, not owned) to coordinate without breaking Rust's borrow rules
- HashMap removal pattern: moving a train out via `.remove()`, calling `depart()`, then re-inserting avoids aliasing mutable references within the HashMap
- Recursive calls via mediator: if departure triggers another arrival (queue processing), carefully manage ownership transfers in/out to avoid double-borrows
