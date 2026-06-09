# Iterator

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/iterator/rust/example](https://refactoring.guru/design-patterns/iterator/rust/example)

## Intent
Implement a stateful iterator over a collection by implementing the Iterator trait and managing position state

## Rust Idiom
Rust implements Iterator as a trait with a required `next()` method that returns `Option<Item>`. Custom iterators hold a mutable index and a borrowed reference to the collection. Unlike OOP inheritance patterns, Rust uses trait implementation and lifetime parameters (`'a`) to ensure the iterator cannot outlive the borrowed collection. Higher-order methods (`map`, `filter`, `fold`) are automatically available once `next()` is defined.

## Key Participants
- UserCollection (the container being iterated)
- UserIterator<'a> (the iterator holding state and a borrowed reference)
- Iterator trait (defines the `next()` contract)
- Option<Item> (signals exhaustion via None)

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::iter::Iterator;

/// A collection that owns its data
pub struct UserCollection {
    users: [&'static str; 3],
}

impl UserCollection {
    pub fn new() -> Self {
        Self {
            users: ["Alice", "Bob", "Carl"],
        }
    }

    /// Returns an iterator borrowing from self
    pub fn iter(&self) -> UserIterator {
        UserIterator {
            index: 0,
            user_collection: self,
        }
    }
}

/// Iterator holding state (index) and a borrowed reference to the collection
pub struct UserIterator<'a> {
    index: usize,
    user_collection: &'a UserCollection,
}

/// Implement the Iterator trait — only next() is required
impl Iterator for UserIterator<'_> {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.user_collection.users.len() {
            let user = self.user_collection.users[self.index];
            self.index += 1;
            Some(user)
        } else {
            None
        }
    }
}

fn main() {
    let collection = UserCollection::new();

    // Manual iteration
    let mut iter = collection.iter();
    println!("1st: {:?}", iter.next()); // Some("Alice")
    println!("2nd: {:?}", iter.next()); // Some("Bob")
    println!("3rd: {:?}", iter.next()); // Some("Carl")
    println!("4th: {:?}", iter.next()); // None

    // Higher-order methods (free from Iterator trait)
    print!("All: ");
    collection.iter().for_each(|u| print!("{} ", u));
    println!();
}
```

## When to Use
- Custom collections where you need to hide internal layout and expose only sequential access
- When a collection structure doesn't match std iterators (e.g., custom graph traversal, lazy computation)
- Implementing multiple independent iteration strategies on the same collection (in_order, reverse, filtered)
- Providing stateful iteration with side effects (e.g., visiting nodes in a tree with context)

## Rust Caveats (ownership / borrow / dispatch)
- The iterator lifetime parameter ('a) is tied to the collection borrow—the iterator cannot outlive the collection reference, preventing use-after-free
- Calling next(&mut self) requires a mutable iterator; immutable iteration requires &self in the trait impl, not mutable state
- The index field must be mutable to change state; if the collection itself is borrowed immutably, only immutable iteration is safe
- If the collection is modified while an iterator is alive, the iterator may access stale indices (no runtime bounds check)—the collection must not mutate during iteration
- IntoIterator (consuming) is different from Iterator (borrowing)—moving the collection into the iterator and yielding owned values requires a different trait impl
