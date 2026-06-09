# Chain of Responsibility

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/chain-of-responsibility/rust/example](https://refactoring.guru/design-patterns/chain-of-responsibility/rust/example)

## Intent
A request passes through a chain of handlers, each deciding to process it or forward to the next handler in the chain.

## Rust Idiom
Rust implements Chain of Responsibility via trait objects (`Box<dyn Trait>`) for dynamic dispatch, where each handler holds an `Option<Box<dyn Handler>>` to the next in sequence. Unlike inheritance-based languages, Rust prefers enum-based dispatch for zero-cost chains at compile time, or trait objects when the chain structure is determined at runtime. Each handler's execute method mutably borrows the request, processes it, and explicitly calls next() to forward control — no implicit delegation.

## Key Participants
- Handler trait: defines handle(&mut self, req: &mut Request) (process this step) and next_mut(&mut self) -> &mut Option<Box<dyn Handler>> (access next link)
- ConcreteHandler structs (StageOne, StageTwo, StageThree): implement Handler, store state and Option<Box<dyn Handler>> for the next handler
- Request struct: the data flowing through the chain, mutated at each step
- execute method on Handler: orchestrates the chain by calling handle() then forwarding via next_mut().execute()

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::fmt;

// Request data structure
#[derive(Default, Clone)]
struct Request {
    name: String,
    stage1_done: bool,
    stage2_done: bool,
    stage3_done: bool,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Handler trait defining the chain contract
trait Handler {
    fn execute(&mut self, req: &mut Request) {
        self.handle(req);
        if let Some(next) = self.next_mut() {
            next.execute(req);
        }
    }

    fn handle(&mut self, req: &mut Request);
    fn next_mut(&mut self) -> &mut Option<Box<dyn Handler>>;
}

// Concrete handlers
struct StageOne {
    next: Option<Box<dyn Handler>>,
}

impl StageOne {
    fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Handler for StageOne {
    fn handle(&mut self, req: &mut Request) {
        if req.stage1_done {
            println!("Stage 1 already done for {}", req);
        } else {
            println!("Stage 1: processing {}", req);
            req.stage1_done = true;
        }
    }

    fn next_mut(&mut self) -> &mut Option<Box<dyn Handler>> {
        &mut self.next
    }
}

struct StageTwo {
    next: Option<Box<dyn Handler>>,
}

impl StageTwo {
    fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Handler for StageTwo {
    fn handle(&mut self, req: &mut Request) {
        if req.stage2_done {
            println!("Stage 2 already done for {}", req);
        } else {
            println!("Stage 2: processing {}", req);
            req.stage2_done = true;
        }
    }

    fn next_mut(&mut self) -> &mut Option<Box<dyn Handler>> {
        &mut self.next
    }
}

struct StageThree {
    next: Option<Box<dyn Handler>>,
}

impl Handler for StageThree {
    fn handle(&mut self, req: &mut Request) {
        if req.stage3_done {
            println!("Stage 3 already done for {}", req);
        } else {
            println!("Stage 3: processing {}", req);
            req.stage3_done = true;
        }
    }

    fn next_mut(&mut self) -> &mut Option<Box<dyn Handler>> {
        &mut self.next
    }
}

impl Default for StageThree {
    fn default() -> Self {
        Self { next: None }
    }
}

fn main() {
    let stage3 = StageThree::default();
    let stage2 = StageTwo::new(stage3);
    let mut stage1 = StageOne::new(stage2);

    let mut req = Request {
        name: "Task-42".into(),
        ..Default::default()
    };

    println!("First pass:");
    stage1.execute(&mut req);

    println!("\nSecond pass (idempotent):");
    stage1.execute(&mut req);
}
```

## When to Use
- Request routing through multiple handlers where the chain is determined at runtime (e.g., HTTP middleware, approval workflows, event dispatching pipelines)
- Processing pipelines where early handlers may short-circuit the chain based on conditions
- Systems where handlers are decoupled and can be reordered without recompilation
- Audit/logging chains where each handler independently observes and optionally modifies the request

## Rust Caveats (ownership / borrow / dispatch)
- Mutable borrow conflicts: next_mut() returns a mutable reference to the Option; calling execute() on the result requires reborrowing, which can fail if the current handler is still borrowed. Rust prevents use-after-move and ensures each handler cannot be traversed twice in a single execution.
- Trait object overhead: Box<dyn Handler> incurs vtable indirection and heap allocation per link. For performance-critical chains, consider enum-based dispatch with pattern matching, which monomorphizes and inlines at compile time.
- Lifetime constraints: If handlers hold references to external state, the trait definition must include explicit lifetimes (trait Handler<'a>), complicating trait object syntax.
- Circular reference risk: Storing a back-reference to the previous handler creates a cycle; Rust's ownership rules force Option<Rc<RefCell<dyn Handler>>>, adding interior mutability and runtime borrow checks. Prefer forward-only chains.
- No implicit default next: Unlike languages with null pointers, Rust requires Option<Box<dyn Handler>>, forcing explicit handling of chain termination. Forgetting to check next_mut() before calling execute() leads to silent short-circuits.
