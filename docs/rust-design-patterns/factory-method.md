# Factory Method

**Category:** Creational
**Source:** [https://refactoring.guru/design-patterns/factory-method/rust/example](https://refactoring.guru/design-patterns/factory-method/rust/example)

## Intent
Defer object creation to subtype implementations via trait methods, enabling polymorphic instantiation without specifying concrete types.

## Rust Idiom
Rust implements Factory Method through trait definitions that declare factory methods returning `Box<dyn Trait>` (dynamic dispatch) or associated types (static dispatch via generics). Unlike OOP inheritance, Rust separates the factory interface (trait) from product instantiation: a trait method returns trait objects, and concrete types implement both the product and creator traits. Generics + associated types provide zero-cost specialization without virtual calls.

## Key Participants
- Button, Dialog traits (product + creator contracts)
- HtmlButton, HtmlDialog and WindowsButton, WindowsDialog (concrete product and creator pairs)
- Box<dyn Button> (trait object for runtime polymorphism)
- MazeGame, Room, MagicMaze, OrdinaryMaze (generic variant using associated types for compile-time specialization)

## Reference Implementation (Rust 2024, compiles standalone)
```rust
// Factory Method: Trait-based creator with factory method returning trait objects

use std::fmt;

// Product trait
pub trait Button: fmt::Display {
    fn render(&self);
    fn on_click(&self);
}

// Creator trait with factory method
pub trait Dialog {
    fn create_button(&self) -> Box<dyn Button>;

    fn render(&self) {
        let button = self.create_button();
        button.render();
    }
}

// Concrete Product 1
pub struct HtmlButton;

impl fmt::Display for HtmlButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HtmlButton")
    }
}

impl Button for HtmlButton {
    fn render(&self) {
        println!("<button>Test</button>");
        self.on_click();
    }

    fn on_click(&self) {
        println!("HTML: Click!");
    }
}

// Concrete Creator 1
pub struct HtmlDialog;

impl Dialog for HtmlDialog {
    fn create_button(&self) -> Box<dyn Button> {
        Box::new(HtmlButton)
    }
}

// Concrete Product 2
pub struct WindowsButton;

impl fmt::Display for WindowsButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WindowsButton")
    }
}

impl Button for WindowsButton {
    fn render(&self) {
        println!("[Windows Button]");
        self.on_click();
    }

    fn on_click(&self) {
        println!("Windows: Click!");
    }
}

// Concrete Creator 2
pub struct WindowsDialog;

impl Dialog for WindowsDialog {
    fn create_button(&self) -> Box<dyn Button> {
        Box::new(WindowsButton)
    }
}

fn main() {
    let dialogs: Vec<Box<dyn Dialog>> = vec![
        Box::new(HtmlDialog),
        Box::new(WindowsDialog),
    ];

    for dialog in dialogs {
        dialog.render();
    }
}
```

## When to Use
- GUI frameworks where different platforms (web, Windows, macOS) need to render the same UI hierarchy with platform-specific widgets
- Plugin systems where the creator knows the family of products but the consumer does not specify which concrete type to instantiate
- Serialization/deserialization where a creator reads a format tag and instantiates the appropriate product type without exposing the variants to the caller
- Testing: swapping concrete implementations via trait objects without recompiling (e.g., MockDatabase vs. RealDatabase)

## Rust Caveats (ownership / borrow / dispatch)
- Box<dyn Trait> incurs heap allocation and vtable indirection on every factory call—for hot paths, use associated types (generics) instead to specialize at compile time with zero runtime cost
- A trait object is unsized; it must always be behind a pointer (Box, &dyn, Rc, Arc). You cannot store Box<dyn Button> in a struct field without making that field a pointer too, changing ownership semantics
- Trait objects can only contain methods that are object-safe: no Self-returning methods, no generic methods, no Self-sized bounds. If a product needs to return Self or be cloned, use generics + associated types, not trait objects
- Circular dependencies emerge if the creator and product both hold trait objects of each other—resolve via Arc<Mutex<>> or refactor to ownership hierarchy (creator owns product, product refs creator via callback or parent pointer)
- Dropping a Box<dyn Trait> calls Drop on the concrete type through the vtable. If the product has cleanup (file handles, locks), ensure the concrete impl handles Drop correctly—generic Drop impls do not infer from trait bounds
