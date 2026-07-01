use super::*;

#[test]
fn test_gates_config_json_deserialize() {
    let json = r#"{
        "$schema": "test",
        "read": { "enabled": true, "blocked_paths": ["/secret"] },
        "bash": { "enabled": false }
    }"#;
    let cfg: GatesConfig = serde_json::from_str(json).expect("parse");
    assert!(cfg.read.enabled);
    assert!(!cfg.bash.enabled);
    assert_eq!(cfg.read.blocked_paths, vec!["/secret"]);
}

#[test]
fn model_autoswitch_defaults_off_and_parses() {
    let cfg: GatesConfig = serde_json::from_str("{}").expect("parse empty");
    assert!(!cfg.model.autoswitch, "autoswitch must default off");
    let on: GatesConfig =
        serde_json::from_str(r#"{ "model": { "autoswitch": true } }"#).expect("parse");
    assert!(on.model.autoswitch, "explicit true parses");
}
