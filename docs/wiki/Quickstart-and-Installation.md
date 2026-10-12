# Quickstart and Installation

Kavach compiles and installs directly into your environment without requiring background database servers or external daemon processes.

---

## 1. One-Line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/Gaurav-Wankhede/kavach-rs/main/install.sh | bash
```

### What the installer does:
1. Bootstraps the pinned Rust toolchain (Edition 2024 via `rustup`).
2. Builds the optimized release binary into `~/.cargo/bin/kavach`.
3. Provisions the sovereign Rust CLI toolbelt (`grep`, `glob`, `json`, `replace`, `script`, `cargo`).
4. Merges deterministic hook definitions into your active AI agent harnesses.

---

## 2. Windows Installation (PowerShell)

```powershell
irm https://raw.githubusercontent.com/Gaurav-Wankhede/kavach-rs/main/install.ps1 | iex
```

---

## 3. Manual Source Build

```bash
git clone https://github.com/Gaurav-Wankhede/kavach-rs.git
cd kavach-rs
cargo build --release -p kavach-cli --bin kavach
cp target/release/kavach ~/.cargo/bin/kavach
kavach --version
```

---

## 4. Upgrading

Kavach includes an in-process updater that syncs with the latest release:

```bash
kavach update
```
