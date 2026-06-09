# Builder

**Category:** Creational
**Source:** [https://refactoring.guru/design-patterns/builder/rust/example](https://refactoring.guru/design-patterns/builder/rust/example)

## Intent
Separate construction logic from product representation using trait-based builders with associated types, allowing flexible product assembly without telescoping constructors.

## Rust Idiom
Rust replaces inheritance-based builders with a generic trait `Builder<T>` using associated types (`type OutputType`), where each concrete builder holds `Option<Field>` fields and validates completeness at build time via `expect()`. The Director pattern pairs with the trait to decouple construction sequences from products. Ownership is key: `build(self)` consumes the builder, preventing accidental reuse, and `&mut self` for setters allows chaining. This is pure trait polymorphism—no vtables, no dynamic dispatch unless you explicitly use `dyn Builder`.

## Key Participants
- Builder trait with associated type OutputType for product type flexibility
- Concrete builder impls (CarBuilder, CarManualBuilder) holding Option<Field> for partial state
- Director orchestrating construction steps uniformly across different builders
- Product types (Car, Manual) with private fields and constructor validation
- Default trait for builder initialization to zero-state

## Reference Implementation (Rust 2024, compiles standalone)
```rust
use std::fmt;

// Product types with private fields
#[derive(Clone, Copy, Debug, PartialEq)]
enum CarType {
    CityCar,
    SportsCar,
    SUV,
}

struct Engine {
    volume: f64,
    mileage: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Transmission {
    Manual,
    Automatic,
    SemiAutomatic,
}

struct Car {
    car_type: CarType,
    seats: u16,
    engine: Engine,
    transmission: Transmission,
    gps: bool,
}

struct Manual {
    car_type: CarType,
    seats: u16,
    engine: Engine,
    transmission: Transmission,
}

impl fmt::Display for Manual {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Manual: {:?}, {} seats, {:?} engine, {:?} transmission",
            self.car_type, self.seats, self.engine.volume, self.transmission
        )
    }
}

// Generic trait with associated output type
trait Builder {
    type Output;
    fn set_car_type(&mut self, car_type: CarType);
    fn set_seats(&mut self, seats: u16);
    fn set_engine(&mut self, engine: Engine);
    fn set_transmission(&mut self, transmission: Transmission);
    fn set_gps(&mut self, gps: bool);
    fn build(self) -> Self::Output; // consumes self, preventing reuse
}

// Concrete builder for Car
#[derive(Default)]
struct CarBuilder {
    car_type: Option<CarType>,
    seats: Option<u16>,
    engine: Option<Engine>,
    transmission: Option<Transmission>,
    gps: Option<bool>,
}

impl Builder for CarBuilder {
    type Output = Car;

    fn set_car_type(&mut self, car_type: CarType) {
        self.car_type = Some(car_type);
    }

    fn set_seats(&mut self, seats: u16) {
        self.seats = Some(seats);
    }

    fn set_engine(&mut self, engine: Engine) {
        self.engine = Some(engine);
    }

    fn set_transmission(&mut self, transmission: Transmission) {
        self.transmission = Some(transmission);
    }

    fn set_gps(&mut self, gps: bool) {
        self.gps = Some(gps);
    }

    fn build(self) -> Car {
        Car {
            car_type: self.car_type.expect("car_type not set"),
            seats: self.seats.expect("seats not set"),
            engine: self.engine.expect("engine not set"),
            transmission: self.transmission.expect("transmission not set"),
            gps: self.gps.unwrap_or(false),
        }
    }
}

// Concrete builder for Manual
#[derive(Default)]
struct ManualBuilder {
    car_type: Option<CarType>,
    seats: Option<u16>,
    engine: Option<Engine>,
    transmission: Option<Transmission>,
}

impl Builder for ManualBuilder {
    type Output = Manual;

    fn set_car_type(&mut self, car_type: CarType) {
        self.car_type = Some(car_type);
    }

    fn set_seats(&mut self, seats: u16) {
        self.seats = Some(seats);
    }

    fn set_engine(&mut self, engine: Engine) {
        self.engine = Some(engine);
    }

    fn set_transmission(&mut self, transmission: Transmission) {
        self.transmission = Some(transmission);
    }

    fn set_gps(&mut self, _: bool) {
        // Manual ignores GPS
    }

    fn build(self) -> Manual {
        Manual {
            car_type: self.car_type.expect("car_type not set"),
            seats: self.seats.expect("seats not set"),
            engine: self.engine.expect("engine not set"),
            transmission: self.transmission.expect("transmission not set"),
        }
    }
}

// Director: orchestrates construction uniformly
struct Director;

impl Director {
    fn build_sports_car<B: Builder>(builder: &mut B) {
        builder.set_car_type(CarType::SportsCar);
        builder.set_seats(2);
        builder.set_engine(Engine {
            volume: 3.0,
            mileage: 0.0,
        });
        builder.set_transmission(Transmission::SemiAutomatic);
        builder.set_gps(true);
    }

    fn build_city_car<B: Builder>(builder: &mut B) {
        builder.set_car_type(CarType::CityCar);
        builder.set_seats(5);
        builder.set_engine(Engine {
            volume: 1.2,
            mileage: 0.0,
        });
        builder.set_transmission(Transmission::Automatic);
        builder.set_gps(true);
    }
}

fn main() {
    // Build a Car via Director
    let mut car_builder = CarBuilder::default();
    Director::build_sports_car(&mut car_builder);
    let car = car_builder.build(); // consumes builder
    println!(
        "Built {:?} car with {} seats, {} L engine",
        car.car_type, car.seats, car.engine.volume
    );

    // Build a Manual via Director
    let mut manual_builder = ManualBuilder::default();
    Director::build_city_car(&mut manual_builder);
    let manual = manual_builder.build(); // consumes builder
    println!("Built manual: {}", manual);

    // Direct builder usage without Director
    let mut custom_car = CarBuilder::default();
    custom_car.set_car_type(CarType::SUV);
    custom_car.set_seats(7);
    custom_car.set_engine(Engine {
        volume: 2.5,
        mileage: 100.0,
    });
    custom_car.set_transmission(Transmission::Automatic);
    let suv = custom_car.build();
    println!(
        "Built custom {:?} with {} seats",
        suv.car_type, suv.seats
    );
}
```

## When to Use
- Complex object construction with many optional or conditional fields where a single constructor would have N combinations of parameters
- Decoupling construction logic from product representation when multiple builders produce different product types (e.g., Car vs Manual documentation)
- Fluent API construction where setters are called in sequence, returning &mut self for chaining (not shown here but idiomatic variant)
- Validation at build time rather than at each setter call, catching incomplete state only when finalizing

## Rust Caveats (ownership / borrow / dispatch)
- build(self) consumes the builder by design—once built, the builder is gone, preventing accidental double-build. Cloning the builder for reuse violates the pattern intent.
- Option<T> fields require expect() or unwrap_or() at build time; no compile-time exhaustiveness check that all required fields were set. Consider a state-machine builder with phantom types (BuilderState) to enforce required fields at compile time.
- Borrowing in Director: &mut self is required for setters, so the builder cannot be borrowed immutably elsewhere. If you need to inspect the builder state mid-construction, either add getter methods or refactor the Director.
- Associated type OutputType ties a builder impl to ONE product type. Multiple products require separate impls (CarBuilder vs ManualBuilder). This is intentional—generics are resolved at compile time, preventing runtime polymorphism unless you use dyn Builder (which loses the associated type).
