# Composite

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/composite/rust/example](https://refactoring.guru/design-patterns/composite/rust/example)

## Intent
Compose objects into tree structures; treat individual items and containers uniformly via a shared trait.

## Rust Idiom
Rust implements Composite via a trait (Component) that both leaf and container types implement. Containers hold `Vec<Box<dyn Trait>>` for heterogeneous polymorphic collections. Recursion happens naturally when a container calls the trait method on its children. Ownership is clean: boxes own the heap-allocated trait objects; mutability flows through `&mut self` on containers that modify children.

## Key Participants
- Component (trait) — unified interface for leaves and containers
- Leaf (File) — implements Component, no children
- Container (Folder) — implements Component, holds Vec<Box<dyn Component>>, delegates operations to children
- Box<dyn Trait> — enables dynamic dispatch and heterogeneous storage

## Reference Implementation (Rust 2024, compiles standalone)
```rust

trait Component {
    fn operation(&self) -> String;
}

struct Leaf {
    name: String,
}

impl Component for Leaf {
    fn operation(&self) -> String {
        format!("Leaf: {}", self.name)
    }
}

struct Composite {
    name: String,
    children: Vec<Box<dyn Component>>,
}

impl Composite {
    fn new(name: impl Into<String>) -> Self {
        Composite {
            name: name.into(),
            children: Vec::new(),
        }
    }

    fn add(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }
}

impl Component for Composite {
    fn operation(&self) -> String {
        let mut result = format!("Composite: {}\n", self.name);
        for child in &self.children {
            result.push_str(&format!("  {}\n", child.operation()));
        }
        result
    }
}

fn main() {
    let mut root = Composite::new("Root");

    let leaf1: Box<dyn Component> = Box::new(Leaf {
        name: "Leaf 1".into(),
    });
    let leaf2: Box<dyn Component> = Box::new(Leaf {
        name: "Leaf 2".into(),
    });

    let mut branch = Composite::new("Branch");
    branch.add(leaf1);
    branch.add(leaf2);

    root.add(Box::new(branch));

    println!("{}", root.operation());
}
```

## When to Use
- Building tree structures (file systems, org charts, UI component hierarchies) where nodes and containers share the same interface
- Implementing DOM-like structures or nested menu systems where you apply operations uniformly to all levels
- Recursive algorithms (rendering, serialization, traversal) that treat containers and leaves identically
- Any recursive data structure where the container is itself a Component and children are heterogeneous

## Rust Caveats (ownership / borrow / dispatch)
- Box<dyn Trait> incurs dynamic dispatch cost; if all children are the same concrete type, use generics + enums instead for zero-cost abstraction
- Mutable access to children requires &mut Composite; shared-ownership scenarios (parent + child pointers) need Rc<RefCell<>> which reintroduces borrow-checker runtime checks
- Lifetime parameters on the trait (e.g., Component<'a>) constrain when and how children can be added; elide them unless external lifetimes are involved
- Trait object coercion (Leaf -> Box<dyn Component>) moves the value onto the heap; moving large leaves into containers is inefficient; consider Arc<> for shared ownership
