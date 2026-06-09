# Template Method

**Category:** Behavioral
**Source:** [https://refactoring.guru/design-patterns/template-method/rust/example](https://refactoring.guru/design-patterns/template-method/rust/example)

## Intent
Define an algorithmic skeleton in a trait, allowing concrete types to override specific steps while keeping the overall structure fixed.

## Rust Idiom
Rust implements Template Method via trait methods that call both concrete methods (required to be implemented by subtypes) and optional hook methods (empty by default). The template method itself is a default trait method that orchestrates the fixed sequence. Instead of inheritance, Rust uses impl blocks for each "concrete class." Generic trait objects or impl Trait parameters let callers abstract over concrete types without runtime dispatch overhead if monomorphization is preferred.

## Key Participants
- TemplateMethod trait — defines template_method() as the fixed skeleton and declares required_operations1/2() as abstract methods and hook1/2() as optional extension points
- ConcreteStruct1, ConcreteStruct2 — types that impl TemplateMethod, providing their own required_operations1/2() bodies while leaving hooks empty or overriding them
- client_code() — generic function accepting impl TemplateMethod, invoking the orchestrator without knowing the concrete type

## Reference Implementation (Rust 2024, compiles standalone)
```rust
trait TemplateMethod {
    fn template_method(&self) {
        self.base_operation1();
        self.required_op1();
        self.base_operation2();
        self.hook1();
        self.required_op2();
        self.hook2();
    }

    fn base_operation1(&self) {
        println!("Base step 1");
    }

    fn base_operation2(&self) {
        println!("Base step 2");
    }

    fn hook1(&self) {}
    fn hook2(&self) {}

    fn required_op1(&self);
    fn required_op2(&self);
}

struct ConcreteA;
impl TemplateMethod for ConcreteA {
    fn required_op1(&self) {
        println!("ConcreteA: operation 1");
    }
    fn required_op2(&self) {
        println!("ConcreteA: operation 2");
    }
}

struct ConcreteB;
impl TemplateMethod for ConcreteB {
    fn required_op1(&self) {
        println!("ConcreteB: operation 1");
    }
    fn required_op2(&self) {
        println!("ConcreteB: operation 2");
    }
    fn hook1(&self) {
        println!("ConcreteB: custom hook");
    }
}

fn execute<T: TemplateMethod>(algo: &T) {
    algo.template_method();
}

fn main() {
    let a = ConcreteA;
    execute(&a);
    println!();
    let b = ConcreteB;
    execute(&b);
}
```

## When to Use
- Multi-step algorithms where the sequence is fixed but each step's implementation varies by type (e.g., data pipeline stages: load → validate → transform → export)
- Framework or library code that defines a standard workflow but lets plugins or user impls fill in domain-specific logic (e.g., request handlers with pre/post hooks)
- Test fixtures or command patterns where setup/teardown is boilerplate but the core operation differs (e.g., database migration with template steps)
- Avoiding code duplication across types that differ only in step implementations, not the orchestration order

## Rust Caveats (ownership / borrow / dispatch)
- Trait objects (dyn TemplateMethod) add runtime dispatch overhead; if concrete types are known at compile time, prefer generic impl Trait to monomorphize and inline the template method
- Required abstract methods force impl blocks to be non-empty; if a step truly has no default, you must still write a body—even if it panic!() or is a no-op, the trait compiler will demand it
- Borrowing: the template_method() default impl takes &self, so all required_op/hook methods must also take &self or &mut self—shared interior mutability (Cell/RefCell) may be needed if a step wants to mutate state without &mut
- Lifetime parameters on the trait propagate to all impls and all call sites; if steps need borrowed data, add explicit lifetimes (e.g., fn required_op1<'a>(&'a self, data: &'a str)) or you will hit the inference cliff
- No sealed trait enforcement in this pattern—callers can impl TemplateMethod downstream; if you want to prevent external impls, use sealed trait via a private module-level trait bound
