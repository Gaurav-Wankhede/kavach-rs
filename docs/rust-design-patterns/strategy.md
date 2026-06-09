# Strategy

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/strategy/rust/example](https://refactoring.guru/design-patterns/strategy/rust/example)

## Intent
Inject interchangeable algorithm families into a context at runtime, allowing clients to swap behavior without changing the context's code.

## Rust Idiom
Rust offers two complementary strategies: (1) **Trait-based generics** — parameterize the context over a trait that each strategy implements, enabling zero-cost abstraction and compile-time dispatch; (2) **Function pointers or closures** — store behavior as fn types or Box<dyn Fn>, trading static dispatch for runtime flexibility. Prefer generics for type safety and performance; use function pointers when strategies are runtime-selected or the context must hold heterogeneous strategies simultaneously.

## Key Participants
- RouteStrategy (trait): contract defining the behavior interface
- WalkingStrategy, PublicTransportStrategy (concrete types): implement the trait
- Navigator<T: RouteStrategy> (context, generic): holds a strategy; monomorphized at compile-time
- FunctionalNavigator (context, runtime): holds fn pointer; dispatches at runtime

## Reference Implementation (Rust 2024, compiles standalone)
```rust
// ============================================================================
// Strategy Pattern (Behavioral) - Rust 2024
// Trait-based + Functional variants demonstrated
// ============================================================================

// --- Trait-Based Strategy (Zero-Cost, Static Dispatch) ---
trait RouteStrategy {
    fn build_route(&self, from: &str, to: &str);
}

struct WalkingStrategy;
impl RouteStrategy for WalkingStrategy {
    fn build_route(&self, from: &str, to: &str) {
        println!("Walk: {} → {} (4 km, 30 min)", from, to);
    }
}

struct PublicTransportStrategy;
impl RouteStrategy for PublicTransportStrategy {
    fn build_route(&self, from: &str, to: &str) {
        println!("Transit: {} → {} (3 km, 5 min)", from, to);
    }
}

// Context holds strategy generically; monomorphized at compile-time
struct Navigator<T: RouteStrategy> {
    strategy: T,
}

impl<T: RouteStrategy> Navigator<T> {
    fn new(strategy: T) -> Self {
        Self { strategy }
    }

    fn route(&self, from: &str, to: &str) {
        self.strategy.build_route(from, to);
    }
}

// --- Functional Strategy (Runtime Dispatch, Flexibility) ---
type RouteStrategyFn = fn(&str, &str);

fn walking(from: &str, to: &str) {
    println!("Walk: {} → {} (4 km, 30 min)", from, to);
}

fn public_transport(from: &str, to: &str) {
    println!("Transit: {} → {} (3 km, 5 min)", from, to);
}

struct FunctionalNavigator {
    strategy: RouteStrategyFn,
}

impl FunctionalNavigator {
    fn new(strategy: RouteStrategyFn) -> Self {
        Self { strategy }
    }

    fn route(&self, from: &str, to: &str) {
        (self.strategy)(from, to);
    }
}

fn main() {
    let nav1 = Navigator::new(WalkingStrategy);
    nav1.route("Home", "Work");

    let nav2 = Navigator::new(PublicTransportStrategy);
    nav2.route("Home", "Work");

    let nav3 = FunctionalNavigator::new(walking);
    nav3.route("Home", "Gym");

    let nav4 = FunctionalNavigator::new(|from, to| {
        println!("Custom: {} → {}", from, to);
    });
    nav4.route("Gym", "Cafe");
}
```

## When to Use
- Selecting between mutually exclusive algorithms at runtime (sorting, compression, routing)
- Reducing conditional branching by delegating to interchangeable implementations
- Testing contexts with mock/stub strategies without modifying production code
- Plugin architectures where strategies are loaded dynamically and stored heterogeneously

## Rust Caveats (ownership / borrow / dispatch)
- Navigator<WalkingStrategy> and Navigator<PublicTransportStrategy> are distinct types; storing both in a Vec requires Box<dyn Trait>, which loses type info and incurs heap allocation
- Trait objects (dyn) add pointer indirection and disable inlining; measure hot paths before committing to dyn dispatch
- Function pointers (fn) capture no state; closures that capture require Box<dyn Fn> and have different move semantics
- Monomorphization bloat: each Navigator<T> generates separate machine code; many strategies inflate binary size
- If strategies hold mutable state or owned data, generics become cumbersome; function pointers decouple ownership but require careful lifetime management
