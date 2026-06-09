# Abstract Factory

**Category:** Creational
**Source:** [https://refactoring.guru/design-patterns/abstract-factory/rust/example](https://refactoring.guru/design-patterns/abstract-factory/rust/example)

## Intent
Create families of related objects (widgets) without specifying their concrete types, delegating instantiation to a trait-bound factory.

## Rust Idiom
Rust uses **associated types** in a factory trait to bind product traits at compile-time (static dispatch via generics), or returns `Box<dyn Trait>` for runtime polymorphism. The pattern decouples client code from concrete product families: pass a factory impl to a generic function, and it manufactures the right family. No virtual-factory base class needed—traits + generics handle the abstraction cleanly.

## Key Participants
- GuiFactory trait with associated types Button, Checkbox
- Button and Checkbox object traits (product interfaces)
- MacFactory, WindowsFactory concrete factories (impl GuiFactory)
- render() generic function (client consuming factories)
- Box<dyn Trait> for runtime polymorphism variant

## Reference Implementation (Rust 2024, compiles standalone)
```rust
// Abstract Factory: Static Dispatch with Associated Types

trait Button {
    fn press(&self);
}

trait Checkbox {
    fn switch(&self);
}

// Factory trait: associated types bind product families at compile-time
trait GuiFactory {
    type B: Button;
    type C: Checkbox;
    fn create_button(&self) -> Self::B;
    fn create_checkbox(&self) -> Self::C;
}

// === Concrete Product Families ===

struct MacButton;
impl Button for MacButton {
    fn press(&self) { println!("Mac button pressed"); }
}

struct MacCheckbox;
impl Checkbox for MacCheckbox {
    fn switch(&self) { println!("Mac checkbox toggled"); }
}

struct MacGUIFactory;
impl GuiFactory for MacGUIFactory {
    type B = MacButton;
    type C = MacCheckbox;
    fn create_button(&self) -> MacButton { MacButton }
    fn create_checkbox(&self) -> MacCheckbox { MacCheckbox }
}

struct WindowsButton;
impl Button for WindowsButton {
    fn press(&self) { println!("Windows button pressed"); }
}

struct WindowsCheckbox;
impl Checkbox for WindowsCheckbox {
    fn switch(&self) { println!("Windows checkbox toggled"); }
}

struct WindowsGUIFactory;
impl GuiFactory for WindowsGUIFactory {
    type B = WindowsButton;
    type C = WindowsCheckbox;
    fn create_button(&self) -> WindowsButton { WindowsButton }
    fn create_checkbox(&self) -> WindowsCheckbox { WindowsCheckbox }
}

// === Client Code (Generic over Factory) ===

fn render<F: GuiFactory>(factory: F) {
    let button = factory.create_button();
    let checkbox = factory.create_checkbox();
    button.press();
    checkbox.switch();
}

fn main() {
    println!("=== macOS ===");
    render(MacGUIFactory);
    
    println!("\n=== Windows ===");
    render(WindowsGUIFactory);
}
```

## When to Use
- Creating UI toolkits for multiple platforms (macOS, Windows, Linux) where widgets differ but families must stay in sync
- Database abstraction layers (Postgres vs SQLite factories producing different connection/pool types)
- Rendering backends (OpenGL vs Vulkan factories creating platform-specific shader/buffer families)
- Microservice SDKs where each backend (AWS, GCP, Azure) is a factory family with consistent product traits

## Rust Caveats (ownership / borrow / dispatch)
- Associated types are resolved at compile-time; the factory trait cannot be used as a dyn trait directly without boxing each product (adding indirection cost). Use `Box<dyn GuiFactory>` only if runtime polymorphism of the factory itself is needed.
- Generic functions `render<F: GuiFactory>` monomorphize at each call site; if you have 10 concrete factories, the compiler generates 10 copies of render()—acceptable for small functions but code bloat for large ones. Box<dyn Trait> avoids monomorphization but pays runtime dispatch overhead.
- Lifetimes: if products hold references to the factory or other state, you must thread lifetimes through GuiFactory methods (e.g., `fn create_button<'a>(&'a self) -> Self::B<'a>`), or use owned types to sidestep the issue.
- Self-referential products are hard: if MacButton wants to hold a &MacGUIFactory, the borrow checker sees Self::B is defined on the same impl block, creating a potential cycle. Reach for owned Rc or Arc if the factory must be accessible from products.
