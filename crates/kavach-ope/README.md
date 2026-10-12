# kavach-ope

Offline Off-Policy Evaluation (OPE) engine for agent governance rules, policy learning, and harness evaluation.

---

## Why kavach-ope Exists

When evolving AI agent governance rules and bandit policies, testing candidate policies directly on live developer workflows risks disruptive false positives and broken builds. 

`kavach-ope` implements statistical Offline Off-Policy Evaluation. Given logged action-propensity-reward samples from previous agent sessions, it estimates the expected value and safety of candidate policies before they are deployed to developer machines.

---

## Key Features

- **Off-Policy Estimation**: Evaluates candidate governance policies using logged historical session records.
- **Importance Sampling**: Standard and weighted importance sampling to correct for distributional shift between logging and target policies.
- **Risk Bounding**: Computes empirical confidence intervals to prevent deploying overly restrictive or unsafe rules.
- **Zero Production Risk**: Completely offline evaluation with zero impact on active agent sessions.

---

## Installation

Add `kavach-ope` to your `Cargo.toml`:

```toml
[dependencies]
kavach-ope = { version = "*", default-features = false }
```

Or via cargo:

```bash
cargo add kavach-ope
```

---

## Behind the Curtain: Honest Disclosures

To maintain total architectural honesty:
- **Offline Statistical Model**: Analyzes logged bandit tuples; does not perform active online reinforcement learning or model updates during agent execution.
- **Data Dependency**: Accuracy is strictly bounded by the coverage and propensity logging of historical session data.
- **Safety**: Pure statistical algorithms implemented in 100% safe Rust.

---

## License

Licensed under the Apache License, Version 2.0.
