use super::advise;

fn tmp(name: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("kavach-lpg-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("src")).expect("mkdir");
    base
}

#[test]
fn rust_without_workspace_lints_advises() {
    let dir = tmp("rust-bare");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    let file = dir.join("src/main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let out = advise(file.to_str().unwrap());
    assert!(out.is_some());
    assert!(out.unwrap().contains("kavach lint init"));
}

#[test]
fn rust_with_workspace_lints_is_clean() {
    let dir = tmp("rust-strict");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname=\"x\"\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
    )
    .unwrap();
    let file = dir.join("src/main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_none());
}

#[test]
fn rust_with_crate_lints_table_is_clean() {
    let dir = tmp("rust-crate-lints");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname=\"x\"\n\n[lints.clippy]\nunwrap_used = \"deny\"\n",
    )
    .unwrap();
    let file = dir.join("src/main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_none());
}

#[test]
fn ts_without_tsconfig_advises() {
    let dir = tmp("ts-bare");
    std::fs::write(dir.join("package.json"), "{}\n").unwrap();
    let file = dir.join("src/app.ts");
    std::fs::write(&file, "export const x = 1\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_some());
}

#[test]
fn ts_with_tsconfig_is_clean() {
    let dir = tmp("ts-cfg");
    std::fs::write(dir.join("package.json"), "{}\n").unwrap();
    std::fs::write(dir.join("tsconfig.json"), "{\"compilerOptions\":{}}\n").unwrap();
    let file = dir.join("src/app.ts");
    std::fs::write(&file, "export const x = 1\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_none());
}

#[test]
fn no_manifest_anywhere_is_clean() {
    let dir = tmp("no-manifest");
    let file = dir.join("src/loose.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_none());
}

#[test]
fn non_source_file_is_clean() {
    let dir = tmp("doc");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    let file = dir.join("README.md");
    std::fs::write(&file, "# x\n").unwrap();
    assert!(advise(file.to_str().unwrap()).is_none());
}
