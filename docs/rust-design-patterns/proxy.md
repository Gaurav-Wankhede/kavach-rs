# Proxy

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/proxy/rust/example](https://refactoring.guru/design-patterns/proxy/rust/example)

## Intent
Provide controlled access to a real object through a proxy that intercepts and enriches requests (rate-limiting, access control, logging).

## Rust Idiom
Rust implements the Proxy pattern via shared traits that both the real object and proxy implement. The proxy wraps the real object by ownership, allowing interception of trait method calls. Unlike OOP inheritance, a generic trait bound lets the proxy work with any conforming type, and methods can be virtual (trait objects with `dyn`) or monomorphic (generics). Rust's ownership model makes the proxy's lifetime and the wrapped object's lifetime explicit in the type system.

## Key Participants
- Server trait: defines the interface (handle_request)
- Application: the real subject implementing Server
- NginxServer: the proxy, holding Application by ownership, also implementing Server
- rate_limiter HashMap: encapsulates the proxy's added control logic

## Reference Implementation (Rust 2024, compiles standalone)
```rust
// Proxy Pattern in Rust: Rate-limiting proxy for a request handler.
// The proxy intercepts requests, checks rate limits, then delegates to the real object.

trait Server {
    fn handle_request(&mut self, url: &str, method: &str) -> (u16, String);
}

// Real subject: the application we want to protect.
struct Application;

impl Server for Application {
    fn handle_request(&mut self, url: &str, method: &str) -> (u16, String) {
        if url == "/app/status" && method == "GET" {
            (200, "Ok".to_string())
        } else if url == "/create/user" && method == "POST" {
            (201, "User Created".to_string())
        } else {
            (404, "Not Found".to_string())
        }
    }
}

// Proxy: wraps Application, intercepts requests, enforces rate limiting.
struct NginxProxy {
    app: Application,
    max_requests: u32,
    counts: std::collections::HashMap<String, u32>,
}

impl NginxProxy {
    fn new(max_requests: u32) -> Self {
        Self {
            app: Application,
            max_requests,
            counts: std::collections::HashMap::new(),
        }
    }

    fn check_rate_limit(&mut self, url: &str) -> bool {
        let count = self.counts.entry(url.to_string()).or_insert(0);
        if *count >= self.max_requests {
            return false;
        }
        *count += 1;
        true
    }
}

impl Server for NginxProxy {
    fn handle_request(&mut self, url: &str, method: &str) -> (u16, String) {
        if !self.check_rate_limit(url) {
            return (429, "Too Many Requests".to_string());
        }
        // Delegate to the real subject.
        self.app.handle_request(url, method)
    }
}

fn main() {
    let mut proxy = NginxProxy::new(2);

    // First two requests succeed.
    let (code, _) = proxy.handle_request("/app/status", "GET");
    println!("Request 1: {}", code); // 200

    let (code, _) = proxy.handle_request("/app/status", "GET");
    println!("Request 2: {}", code); // 200

    // Third request to same URL is rate-limited.
    let (code, body) = proxy.handle_request("/app/status", "GET");
    println!("Request 3: {} - {}", code, body); // 429 - Too Many Requests

    // Different URL has its own limit.
    let (code, _) = proxy.handle_request("/create/user", "POST");
    println!("Request 4 (different URL): {}", code); // 201
}
```

## When to Use
- Rate-limiting or throttling access to an expensive service (database, API, file I/O) by wrapping it in a proxy that enforces quotas before delegation.
- Lazy initialization: proxy holds an Option<RealSubject> and instantiates it only on first use.
- Access control: proxy checks permissions, authentication, or roles before forwarding to the protected object.
- Logging/monitoring: proxy records all requests and responses, adding observability without changing the real object.

## Rust Caveats (ownership / borrow / dispatch)
- The proxy takes ownership of the real object (self.app: Application), not a reference; if you need the real object elsewhere or in multiple proxies, use Rc<RefCell<T>> or Arc<Mutex<T>> for shared, interior-mutable access—but this introduces runtime borrow-check overhead and potential deadlocks.
- Trait objects (dyn Server) allow runtime polymorphism but disable static dispatch; use generic proxies (struct Proxy<T: Server>) to inline the real object's methods at compile-time, keeping the proxy zero-cost.
- The proxy's &mut self requirement is contagious: any method that calls handle_request requires &mut, even if the real object's implementation is read-only. Consider using interior mutability (RefCell, Mutex) for the HashMap if the proxy is shared immutably.
- Lifetimes matter: if the proxy holds a reference to the real object instead of ownership, the proxy's lifetime is bounded by the real object's; ownership (owned field) is simpler but prevents the real object from being used independently while the proxy exists.
