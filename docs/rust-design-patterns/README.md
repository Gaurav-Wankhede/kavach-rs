# Rust Design Patterns — Strict Reference

Idiomatic Rust 2024 implementations of the Gang-of-Four patterns, crawled and synthesized from [refactoring.guru/design-patterns/rust](https://refactoring.guru/design-patterns/rust).

Each card carries the Rust-specific idiom (traits/enums/ownership over inheritance), a self-contained compilable example, and the borrow-checker caveats unique to that pattern in Rust.


## Creational

- [**Abstract Factory**](abstract-factory.md) — Create families of related objects (widgets) without specifying their concrete types, delegating instantiation to a trait-bound factory.
- [**Builder**](builder.md) — Separate construction logic from product representation using trait-based builders with associated types, allowing flexible product assembly without telescoping constructors.
- [**Factory Method**](factory-method.md) — Defer object creation to subtype implementations via trait methods, enabling polymorphic instantiation without specifying concrete types.
- [**Prototype**](prototype.md) — Enable object creation by cloning a prototype instance rather than constructing from scratch, leveraging Rust's Clone trait for type-safe, zero-cost duplication.
- [**Singleton**](singleton.md) — Provide thread-safe, lazy-initialized global state without runtime panics or external crates using std::sync::OnceLock.

## Structural

- [**Adapter**](adapter.md) — Wrap an incompatible interface inside a compatible trait to enable collaboration without modifying the adaptee.
- [**Bridge**](bridge.md) — Decouple an abstraction from its implementation so the two can vary independently, using trait composition and generics.
- [**Composite**](composite.md) — Compose objects into tree structures; treat individual items and containers uniformly via a shared trait.
- [**Decorator**](decorator.md) — Dynamically add responsibilities to an object at runtime by wrapping it in a struct that implements the same trait.
- [**Facade**](facade.md) — Provide a simplified, unified interface to a set of subsystems, hiding their complexity behind a single facade type.
- [**Flyweight**](flyweight.md) — Share expensive immutable intrinsic state among many objects via a factory, storing only cheap extrinsic state per instance to reduce memory footprint.
- [**Proxy**](proxy.md) — Provide controlled access to a real object through a proxy that intercepts and enriches requests (rate-limiting, access control, logging).

## Behavioral

- [**Chain of Responsibility**](chain-of-responsibility.md) — A request passes through a chain of handlers, each deciding to process it or forward to the next handler in the chain.
- [**Command**](command.md) — Encapsulate a request as an object, parameterize invokers with commands, queue/undo, and decouple senders from receivers.
- [**Iterator**](iterator.md) — Implement a stateful iterator over a collection by implementing the Iterator trait and managing position state
- [**Mediator**](mediator.md) — Centralize object interactions through a mediator to reduce coupling between components and decouple business logic from coordination.
- [**Memento**](memento.md) — Capture object state without exposing internal representation, enabling undo/restore without breaking encapsulation.
- [**Observer**](observer.md) — Implement the Observer pattern in Rust using function pointers and HashMap to decouple event publishers from subscribers.
- [**State**](state.md) — Encapsulate state-dependent behavior into objects that implement a common trait, allowing state transitions without conditionals.
- [**Strategy**](strategy.md) — Inject interchangeable algorithm families into a context at runtime, allowing clients to swap behavior without changing the context's code.
- [**Template Method**](template-method.md) — Define an algorithmic skeleton in a trait, allowing concrete types to override specific steps while keeping the overall structure fixed.
- [**Visitor**](visitor.md) — Define how a computation/operation traverses a heterogeneous data structure by delegating to type-specific visitor implementations, decoupling algorithms from data representation.
