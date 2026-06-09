# Flyweight

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/flyweight/rust/example](https://refactoring.guru/design-patterns/flyweight/rust/example)

## Intent
Share expensive immutable intrinsic state among many objects via a factory, storing only cheap extrinsic state per instance to reduce memory footprint.

## Rust Idiom
Rust implements Flyweight via an immutable factory that caches and returns references to shared TreeType objects. The pattern leverages Rust's ownership model: the factory owns the Vec of TreeTypes, callers get &TreeType references (not copies), and extrinsic state (coordinates) lives separately on each context (Tree). No Clone or Arc overhead because references are zero-cost; the borrow checker ensures the factory outlives all references.

## Key Participants
- TreeType (intrinsic flyweight struct holding shared immutable data)
- TreeFactory (singleton-like cache managing TreeType instances)
- Tree (context struct pairing extrinsic state x,y with &TreeType reference)
- Client code drawing many trees efficiently by reusing TreeType references

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::collections::HashMap;

/// Intrinsic state: shared immutable data (expensive to duplicate)
struct TreeType {
    name: String,
    texture: String,
}

impl TreeType {
    fn new(name: &str, texture: &str) -> Self {
        TreeType {
            name: name.to_string(),
            texture: texture.to_string(),
        }
    }

    fn draw(&self, x: i32, y: i32) {
        println!(
            "Drawing {} at ({}, {}) with texture {}",
            self.name, x, y, self.texture
        );
    }
}

/// Flyweight factory: caches and reuses TreeType instances
struct TreeFactory {
    tree_types: HashMap<String, TreeType>,
}

impl TreeFactory {
    fn new() -> Self {
        TreeFactory {
            tree_types: HashMap::new(),
        }
    }

    /// Populate phase: insert (or keep) a flyweight under `key`. Takes &mut self.
    fn register(&mut self, key: &str, name: &str, texture: &str) {
        self.tree_types
            .entry(key.to_string())
            .or_insert_with(|| TreeType::new(name, texture));
    }

    /// Read phase: hand out a shared reference. Takes &self, so many can coexist.
    fn get(&self, key: &str) -> &TreeType {
        self.tree_types.get(key).expect("flyweight not registered")
    }
}

/// Context: extrinsic state (position) paired with shared flyweight reference
struct Tree<'a> {
    x: i32,
    y: i32,
    tree_type: &'a TreeType,
}

impl<'a> Tree<'a> {
    fn new(x: i32, y: i32, tree_type: &'a TreeType) -> Self {
        Tree { x, y, tree_type }
    }

    fn draw(&self) {
        self.tree_type.draw(self.x, self.y);
    }
}

fn main() {
    let mut factory = TreeFactory::new();

    // Populate phase: all &mut borrows finish before any shared ref is taken.
    factory.register("pine", "Pine", "pine_texture.png");
    factory.register("oak", "Oak", "oak_texture.png");

    // Read phase: shared &TreeType references can now coexist freely.
    let pine_ref = factory.get("pine");
    let oak_ref = factory.get("oak");

    let tree1 = Tree::new(10, 20, pine_ref);
    let tree2 = Tree::new(50, 60, pine_ref); // Reuses same TreeType
    let tree3 = Tree::new(100, 150, oak_ref);

    tree1.draw();
    tree2.draw();
    tree3.draw();

    println!("Cached {} unique TreeTypes", factory.tree_types.len());
}
```

## When to Use
- Rendering thousands of game objects (trees, particles, buildings) where most share immutable visual/model data but differ in position/rotation/health
- Text editors storing character metadata (font, color) once per style but positioning per glyph instance
- Web servers caching immutable stylesheet/image metadata while storing per-request extrinsic state (URL, timestamp, client IP) separately
- Memory-constrained systems (embedded, IoT) where object count is high but intrinsic state dominates allocation budget

## Rust Caveats (ownership / borrow / dispatch)
- Two-phase borrow is mandatory: a factory method that takes `&mut self` (to insert) AND returns `&TreeType` poisons the well — every returned ref keeps the `&mut` alive, so a second lookup fails E0499. Split into a `register(&mut self)` populate phase and a `get(&self)` read phase; all mutation finishes before any shared ref is handed out.
- Lifetime tie between Tree and factory: `&'a TreeType` forces the factory to outlive every Tree; drop the factory first and the refs dangle (the borrow checker rejects it). If the factory's lifetime is unclear, switch the cache to `Arc<TreeType>` and store owned `Arc`s in each Tree.
- Thread safety: `HashMap` access is `&mut` for insert and the map is neither `Send`-shared nor lock-free; for concurrent flyweight creation use `DashMap` or `Arc<Mutex<HashMap<..>>>`, or pre-populate at startup and share `&`.
- Clone defeats the pattern: if you `clone()` a `TreeType` into each Tree instead of borrowing, you pay the per-instance memory the flyweight exists to avoid — and nothing in the type system warns you when the intrinsic state is cheap to copy.
