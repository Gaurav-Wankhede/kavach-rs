# Decorator

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/decorator/rust/example](https://refactoring.guru/design-patterns/decorator/rust/example)

## Intent
Dynamically add responsibilities to an object at runtime by wrapping it in a struct that implements the same trait.

## Rust Idiom
Rust implements Decorator via trait composition and newtype wrappers. Instead of inheritance chains, a wrapper struct holds a trait object (or generic type) and implements the same trait, forwarding calls and adding behavior. No vtables required for monomorphic paths; generic decorators inline at compile time. For runtime-polymorphic decorators, use `Box<dyn Trait>` to enable mixing decorator layers.

## Key Participants
- Read trait: the component interface that both the concrete type and decorators implement
- Cursor<&str>: the concrete component being decorated
- BufReader<T>: the decorator wrapping T and adding buffering behavior
- Box<dyn Read>: trait object enabling runtime decorator stacking

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::io::{self, Read, BufReader, Cursor};

// Component trait: the interface both concrete and decorated types satisfy.
trait Component: Read {
    fn operation(&self) -> String;
}

// Concrete component.
struct DataSource {
    data: String,
}

impl Read for DataSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes = self.data.as_bytes();
        let n = std::cmp::min(buf.len(), bytes.len());
        buf[..n].copy_from_slice(&bytes[..n]);
        self.data = self.data[n..].to_string();
        Ok(n)
    }
}

impl Component for DataSource {
    fn operation(&self) -> String {
        format!("DataSource: {}", self.data)
    }
}

// Decorator: wraps a component and adds behavior.
struct LoggingDecorator<C: Component> {
    component: C,
}

impl<C: Component + Read> Read for LoggingDecorator<C> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        eprintln!("[LOG] Reading {} bytes", buf.len());
        self.component.read(buf)
    }
}

impl<C: Component> Component for LoggingDecorator<C> {
    fn operation(&self) -> String {
        format!("[Logged] {}", self.component.operation())
    }
}

// Runtime-polymorphic decorator using trait object.
struct EncryptionDecorator {
    inner: Box<dyn Read>,
}

impl Read for EncryptionDecorator {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

fn main() {
    let data = DataSource { data: "secret".to_string() };
    
    // Monomorphic decorator stacking via generics.
    let logged = LoggingDecorator { component: data };
    println!("{}", logged.operation());
    
    // Runtime polymorphic: stack decorators via trait objects.
    let input = BufReader::new(Cursor::new("buffered data"));
    let mut encrypted: Box<dyn Read> = Box::new(input);
    
    let mut buf = [0u8; 11];
    let _ = encrypted.read(&mut buf);
    println!("Read: {}", String::from_utf8_lossy(&buf[..]));
}
```

## When to Use
- Adding features to an object without modifying its source: logging, encryption, compression wrappers around I/O.
- Runtime composition of behaviors: stacking decorators chosen at runtime based on config or input.
- Avoiding deep inheritance hierarchies: Rust has no inheritance; decorators compose linearly.
- Trait-based API design: any type implementing Read can be wrapped, preserving trait bounds.

## Rust Caveats (ownership / borrow / dispatch)
- Trait object overhead: Box<dyn Trait> uses dynamic dispatch (vtable lookup); generic decorators are monomorphic and inline, but require knowing all types at compile time.
- Borrowing across layers: a decorator borrows the wrapped component; the borrow checker prevents self-referential chains unless you use Rc/Arc.
- Read trait limitations: Read::read takes &mut self, so chaining mutable borrows requires the outer decorator to own or mutably borrow the inner one; immutable forwarding (via &self on a read-only interface) is simpler.
- Generic vs trait object tradeoff: a generic decorator <C: Component> inlines at compile time (code bloat, no cost at runtime); a Box<dyn Trait> is smaller (one vtable) but pays dispatch cost on each call.
- Lifetime bounds: if the component holds references, the decorator must preserve them; use explicit lifetime parameters on the decorator struct to avoid 'static assumptions.
