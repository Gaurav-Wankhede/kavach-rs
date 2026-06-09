# Facade

**Category:** Structural
**Source:** [https://refactoring.guru/design-patterns/facade/rust/example](https://refactoring.guru/design-patterns/facade/rust/example)

## Intent
Provide a simplified, unified interface to a set of subsystems, hiding their complexity behind a single facade type.

## Rust Idiom
Rust implements Facade via a struct that holds instances (or references) to subsystem components and exposes high-level methods that orchestrate calls to those components. No inheritance; instead, composition and trait bounds (if needed) enforce the contract. The facade owns or borrows its subsystems, delegating to their methods while managing error propagation via Result.

## Key Participants
- WalletFacade: the high-level unified interface, owns Account, Wallet, SecurityCode, Notification, and Ledger
- Account, Wallet, SecurityCode, Notification, Ledger: subsystem components with focused responsibilities
- Result: error propagation from subsystems through the facade's public API

## Reference Implementation (Rust 2024, compiles standalone)
```rust
// Facade Design Pattern in Rust (Structural)
// A unified interface hiding multiple subsystem components.

// Subsystem components
struct Account {
    name: String,
}

impl Account {
    fn new(name: String) -> Self {
        Self { name }
    }

    fn check(&self, name: &str) -> Result<(), String> {
        if self.name != name {
            return Err("Account mismatch".into());
        }
        println!("Account verified");
        Ok(())
    }
}

struct Wallet {
    balance: u32,
}

impl Wallet {
    fn new() -> Self {
        Self { balance: 0 }
    }

    fn credit(&mut self, amount: u32) {
        self.balance += amount;
    }

    fn debit(&mut self, amount: u32) -> Result<(), String> {
        if self.balance < amount {
            return Err("Insufficient balance".into());
        }
        self.balance -= amount;
        Ok(())
    }
}

struct SecurityCode(u32);

impl SecurityCode {
    fn verify(&self, code: u32) -> Result<(), String> {
        if self.0 != code {
            return Err("Invalid security code".into());
        }
        println!("Security code verified");
        Ok(())
    }
}

// The Facade: simplifies the API by wrapping subsystems
struct WalletFacade {
    account: Account,
    wallet: Wallet,
    code: SecurityCode,
}

impl WalletFacade {
    fn new(account_name: String, security_code: u32) -> Self {
        println!("Initializing wallet facade...");
        Self {
            account: Account::new(account_name),
            wallet: Wallet::new(),
            code: SecurityCode(security_code),
        }
    }

    // High-level interface: clients call one method instead of coordinating subsystems
    fn add_money(&mut self, account: &str, code: u32, amount: u32) -> Result<(), String> {
        self.account.check(account)?;
        self.code.verify(code)?;
        self.wallet.credit(amount);
        println!("Added {} to wallet", amount);
        Ok(())
    }

    fn remove_money(&mut self, account: &str, code: u32, amount: u32) -> Result<(), String> {
        self.account.check(account)?;
        self.code.verify(code)?;
        self.wallet.debit(amount)?;
        println!("Removed {} from wallet", amount);
        Ok(())
    }
}

fn main() -> Result<(), String> {
    let mut facade = WalletFacade::new("alice".into(), 1234);
    
    facade.add_money("alice", 1234, 100)?;
    facade.remove_money("alice", 1234, 30)?;
    
    Ok(())
}
```

## When to Use
- Simplifying a complex subsystem API: wrap multiple interdependent components behind a single high-level interface.
- Decoupling clients from subsystem internals: clients only depend on the facade, not the individual subsystems.
- Layered architectures: a facade acts as the entry point to a layer (e.g., a service layer wrapping data access, validation, and logging).
- Orchestrating multi-step workflows: the facade coordinates sequenced calls across subsystems, centralizing orchestration logic.

## Rust Caveats (ownership / borrow / dispatch)
- Ownership in the facade: if subsystems are moved into the facade (owned), the facade must have exclusive control; borrowing requires lifetime parameters and careful reference management.
- Error propagation: the facade's Result type must propagate all subsystem errors; mixing error types requires a unified error enum or Into trait implementation.
- Mutability: if subsystems require &mut self, the facade must also be &mut; this can conflict with shared-state scenarios (use interior mutability like RefCell if needed).
- Generic subsystems: if you want the facade to work with different implementations of a subsystem, bind it to a trait; this requires dynamic dispatch (dyn Trait) or generics, adding complexity.
