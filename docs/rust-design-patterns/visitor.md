# Visitor

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/visitor/rust/example](https://refactoring.guru/design-patterns/visitor/rust/example)

## Intent
Define how a computation/operation traverses a heterogeneous data structure by delegating to type-specific visitor implementations, decoupling algorithms from data representation.

## Rust Idiom
Rust implements Visitor via trait objects or generic trait bounds instead of inheritance. The key difference: instead of classes inheriting from an abstract Visitor base, concrete types *implement* a Visitor trait with an associated type (`type Value`). The "visitable" types call visitor methods, which dispatch to the right impl via trait coherence or enum pattern matching. This avoids runtime polymorphism cost (no vtable lookup) when using generic bounds, but allows it via `dyn Trait` when heterogeneous collections are required.

## Key Participants
- Visitor trait: defines visit_* methods returning an associated type Value
- Concrete visitor impl: e.g., TwoValuesStruct implementing Visitor with a concrete Value type
- Visitable type (Element): calls visitor.visit_X(self) to delegate its transformation
- Enum or trait object: holds heterogeneous elements; iterator feeds each to the visitor

## Reference Implementation (Rust 2024, compiles standalone)
```rust

// Minimal Rust Visitor Pattern (2024)
// Decouples operations (visitors) from data structure shape.

use std::fmt;

// The Visitor trait: defines operations on different data types.
trait Visitor {
    type Output;
    fn visit_int(&mut self, val: i32) -> Self::Output;
    fn visit_string(&mut self, val: &str) -> Self::Output;
}

// Concrete visitor: sums integers, counts string chars.
struct SumAndCountVisitor {
    sum: i32,
    char_count: usize,
}

impl Visitor for SumAndCountVisitor {
    type Output = String;

    fn visit_int(&mut self, val: i32) -> Self::Output {
        self.sum += val;
        format!("Sum now: {}", self.sum)
    }

    fn visit_string(&mut self, val: &str) -> Self::Output {
        self.char_count += val.len();
        format!("Char count now: {}", self.char_count)
    }
}

// Another visitor: converts to JSON repr.
struct JsonVisitor;

impl Visitor for JsonVisitor {
    type Output = String;

    fn visit_int(&mut self, val: i32) -> Self::Output {
        format!("{{\"type\":\"int\",\"value\":{}}}", val)
    }

    fn visit_string(&mut self, val: &str) -> Self::Output {
        format!("{{\"type\":\"string\",\"value\":\"{}\"}}", val)
    }
}

// Heterogeneous element enum.
enum Element {
    Int(i32),
    Str(String),
}

impl Element {
    fn accept<V: Visitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Element::Int(n) => visitor.visit_int(*n),
            Element::Str(s) => visitor.visit_string(s),
        }
    }
}

fn main() {
    let elements = vec![
        Element::Int(10),
        Element::Str("hello".to_string()),
        Element::Int(5),
    ];

    // Run SumAndCountVisitor
    let mut sum_visitor = SumAndCountVisitor { sum: 0, char_count: 0 };
    for elem in &elements {
        let result = elem.accept(&mut sum_visitor);
        println!("{}", result);
    }

    // Run JsonVisitor
    let mut json_visitor = JsonVisitor;
    for elem in &elements {
        let json = elem.accept(&mut json_visitor);
        println!("{}", json);
    }
}
```

## When to Use
- AST traversal: compiler analysis needs multiple passes (type-checking, optimization, code-gen) without modifying the AST structure each time
- Report generation from heterogeneous data: same data structure feeds different exporters (CSV, JSON, PDF) via separate visitors
- Multi-format deserialization (Serde-style): a Visitor trait lets each target type define how it consumes raw input without coupling the deserializer to all target types
- Double dispatch by hand: when you need to dispatch on two types (element + visitor operation) and Rust's single-dispatch rules would otherwise force pattern matching in the operation itself

## Rust Caveats (ownership / borrow / dispatch)
- Associated type coupling: if Visitor::Output differs per impl, callers must know the concrete type or use `dyn Trait + where dyn Trait: Visitor<Output = T>` — this bounds the output type globally, narrowing reuse
- Mutability of visitor state: visitor must be `&mut self` if it accumulates state (e.g., sum_visitor); immutable visits are harder with borrowed state; &mut rules force careful lifetime management
- Enum exhaustiveness: every concrete Element variant must be handled in accept's match; adding a new variant requires touching every visitor impl, breaking encapsulation—consider trait-based heterogeneity (`dyn Element`) to soften this
- dyn Trait overhead: if elements are stored as `Box<dyn Element>`, dispatch is a vtable lookup per element; generic-bounded visitors avoid this but require monomorphization and are less flexible for truly dynamic collections
- Visitor lifetime: if visitor holds borrowed data (e.g., `&'a mut Context`), the borrow must outlive all element.accept() calls; Rust forces you to prove this or restructure to owned state
