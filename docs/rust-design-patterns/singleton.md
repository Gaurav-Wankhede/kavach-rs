# Singleton

**Category:** Creational
**Source:** [https://refactoring.guru/design-patterns/singleton/rust/example](https://refactoring.guru/design-patterns/singleton/rust/example)

## Intent
Provide thread-safe, lazy-initialized global state without runtime panics or external crates using std::sync::OnceLock.

## Rust Idiom
Rust singletons avoid the OOP "global mutable object" anti-pattern by preferring immutable statics (`OnceLock`, `const`) or passing references. When global state is unavoidable, `OnceLock<T>` (Rust 1.70+) provides zero-cost, panic-free lazy initialization without `unsafe` or external crates. For truly mutable global state, `Mutex<T>` or `RwLock<T>` wraps the inner type, but requires lock acquisition on every access—preferred only when mutation is the true requirement, not just a legacy habit.

## Key Participants
- OnceLock<T> — lazy, thread-safe initialization cell holding one T forever; get() returns Option, get_or_init() forces init on first call
- Mutex<T> — interior-mutable guard for thread-safe shared mutation; lock() waits, unwrap() panics if poisoned
- fn(T) -> T — the init closure; must be pure (deterministic, side-effect-free) for soundness
- static — compile-time singleton anchor; Rust requires sync/send bounds on the held type

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::sync::{OnceLock, Mutex};

// Pattern 1: Immutable lazy singleton (preferred)
fn database() -> &'static str {
    static DB: OnceLock<String> = OnceLock::new();
    DB.get_or_init(|| {
        eprintln!("Initializing database connection...");
        "postgres://localhost".to_string()
    })
}

// Pattern 2: Mutable lazy singleton (only if mutation is required)
fn shared_state() -> &'static Mutex<Vec<i32>> {
    static STATE: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(Vec::new()))
}

// Pattern 3: Custom type with OnceLock
struct Config {
    host: String,
    port: u16,
}

impl Config {
    fn global() -> &'static Config {
        static CFG: OnceLock<Config> = OnceLock::new();
        CFG.get_or_init(|| Config {
            host: "0.0.0.0".to_string(),
            port: 8080,
        })
    }
}

fn main() {
    // Immutable singleton: zero-cost after first call
    let db1 = database();
    let db2 = database(); // returns cached reference, no re-init
    assert_eq!(db1, db2);
    println!("DB: {}", db1);

    // Mutable singleton: requires lock
    {
        let mut state = shared_state().lock().unwrap();
        state.push(42);
    }
    {
        let state = shared_state().lock().unwrap();
        println!("State: {:?}", *state);
    }

    // Custom config singleton
    let cfg = Config::global();
    println!("Config: {}:{}", cfg.host, cfg.port);
}
```

## When to Use
- Global database connection pools or runtime configuration that is initialized once and shared read-only across the app
- Shared mutable state in multi-threaded servers (e.g., a Mutex-wrapped request counter or log buffer)
- Lazy resource acquisition on first use (TLDR: files, network sockets, expensive parsers) to avoid startup latency
- Dependency injection roots where a factory function is not practical (rare; prefer passing references or DI containers instead)

## Rust Caveats (ownership / borrow / dispatch)
- OnceLock::get_or_init() is NOT reentrant: if the closure tries to call get_or_init() again on the same static, it will deadlock or panic; design the init logic to be pure and independent
- Mutex::lock().unwrap() panics if the lock is poisoned (another thread panicked while holding it); in production, handle the Err case or use a poison-recovery strategy
- OnceLock<T> requires T: Sync + Send; if T contains !Send types (e.g., Rc<_>, raw pointers), the static will not compile; use thread_local!() or RefCell in a non-shared context instead
- Every access to Mutex<T> acquires the lock; if hot-path reads outnumber writes 100:1, RwLock<T> may be faster, but adds complexity and risk of reader starvation—measure first
- Once initialized, OnceLock<T> holds T forever; if you need cleanup (close a file), that happens at program exit via normal Drop rules, not a destructor you control
