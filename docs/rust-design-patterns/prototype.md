# Prototype

**Category:** Creational
**Source:** [https://refactoring.guru/design-patterns/prototype/rust/example](https://refactoring.guru/design-patterns/prototype/rust/example)

## Intent
Enable object creation by cloning a prototype instance rather than constructing from scratch, leveraging Rust's Clone trait for type-safe, zero-cost duplication.

## Rust Idiom
Rust implements Prototype via the built-in Clone trait, avoiding the manual copy constructors and vtable-based virtual clone() methods of C++/Java. Derive Clone for value types with shallow structure, implement it manually for complex types with deep semantics, or wrap types in Rc<RefCell<T>> / Arc<Mutex<T>> for shared ownership during cloning. No factory pattern required—clone() is the prototype operation.

## Key Participants
- Clone trait: standard library marker trait enabling bitwise or custom duplication; automatically derived for Copy types
- Rc<RefCell<T>>: shared ownership without garbage collection; enables cloning of interior-mutable objects when Clone is not auto-derivable
- Arc<Mutex<T>>: thread-safe shared ownership; required for cloning concurrent prototypes across threads
- Prototype (concrete type): any struct with Clone impl becomes a blueprint for new instances via .clone()

## Reference Implementation (Rust 2024, compiles standalone)
```rust

use std::cell::RefCell;
use std::rc::Rc;

/// A simple prototype with value semantics.
#[derive(Clone)]
struct Circle {
    pub x: u32,
    pub y: u32,
    pub radius: u32,
}

/// A prototype with interior-mutable, mutable state.
#[derive(Clone)]
struct ComplexShape {
    pub name: String,
    state: Rc<RefCell<ShapeState>>,
}

#[derive(Clone)]
struct ShapeState {
    pub color: String,
    pub rotated: bool,
}

impl ComplexShape {
    fn new(name: &str, color: &str) -> Self {
        ComplexShape {
            name: name.to_string(),
            state: Rc::new(RefCell::new(ShapeState {
                color: color.to_string(),
                rotated: false,
            })),
        }
    }

    fn describe(&self) -> String {
        let s = self.state.borrow();
        format!(
            "{}: color={}, rotated={}",
            self.name, s.color, s.rotated
        )
    }
}

fn main() {
    // Simple prototype: derive Clone on a value type.
    let circle1 = Circle {
        x: 10,
        y: 15,
        radius: 10,
    };
    let mut circle2 = circle1.clone();
    circle2.radius = 77;

    println!("Circle 1: x={}, y={}, r={}", circle1.x, circle1.y, circle1.radius);
    println!("Circle 2: x={}, y={}, r={}", circle2.x, circle2.y, circle2.radius);

    // Complex prototype: Rc<RefCell<T>> allows shared-state cloning.
    let shape1 = ComplexShape::new("Square", "red");
    let shape2 = shape1.clone();

    println!("\n{}", shape1.describe());
    println!("{}", shape2.describe());

    // Mutate via interior mutability; both shapes see the change.
    shape1.state.borrow_mut().color = "blue".to_string();
    println!("\nAfter mutation:");
    println!("{}", shape1.describe());
    println!("{}", shape2.describe());
}
```

## When to Use
- Creating many similar objects with slight variations (e.g., UI widgets from a template, game entities from a prefab)
- Avoiding expensive re-initialization when a partially-configured object can be cheaply duplicated
- Building undo/redo systems by cloning state snapshots before mutations
- Implementing object pools or caches where the cost of Clone is lower than construction

## Rust Caveats (ownership / borrow / dispatch)
- Clone creates a shallow copy by default; deep copies of nested Rc or Arc require manual traversal—cloning Rc<T> clones the pointer, not the T inside
- Derive Clone only if all fields are Clone; missing Clone on a single field blocks auto-derivation—no fallback impl
- Rc<RefCell<T>> clones share the same interior state; mutations via one clone are visible to all—this is intentional for shared prototypes but breaks isolation if unintended
- Cloning behind a lock (Arc<Mutex<T>>) may panic if another thread poisons the Mutex; wrap clone() in a critical section if needed
- Generic Clone over dyn Trait requires explicit type knowledge at call site—if you need cloning of dynamic types, use Rc or Arc with Clone-object wrappers, not bare trait objects
