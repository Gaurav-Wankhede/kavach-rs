use super::install::install_binary;
use super::lock::DeployLock;
use std::fs;

#[test]
fn replaces_a_dangling_symlink() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = std::env::temp_dir().join(format!("kavach-dangletest-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src-bin");
    fs::write(&src, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();
    let dst = dir.join("kavach");
    std::os::unix::fs::symlink(dir.join("does-not-exist"), &dst).unwrap();
    assert!(
        !dst.exists(),
        "precondition: dangling symlink (exists() follows → false)"
    );

    install_binary(&src, &dst).expect("must replace dangling symlink with the new file");
    let ft = dst.symlink_metadata().unwrap().file_type();
    assert!(!ft.is_symlink() && !ft.is_dir());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn deploy_concurrent_lock_prevents_race() {
    use super::lock::DeployLock;

    let dir = std::env::temp_dir().join(format!("kavach-locktest-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let first = DeployLock::try_acquire(&dir)
        .expect("open lock file")
        .expect("first acquire must win");

    let second = DeployLock::try_acquire(&dir).expect("open lock file");
    assert!(
        second.is_none(),
        "second concurrent deploy must be refused while the lock is held"
    );

    drop(first);
    let third = DeployLock::try_acquire(&dir)
        .expect("open lock file")
        .expect("re-acquire must win after the prior guard drops");
    drop(third);

    fs::remove_dir_all(&dir).ok();
}
