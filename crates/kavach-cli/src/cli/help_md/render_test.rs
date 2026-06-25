use super::arg_row;

#[test]
fn long_flag_with_help_and_default() {
    let a = clap::Arg::new("port")
        .long("port")
        .help("TCP port")
        .default_value("7777");
    assert_eq!(arg_row(&a), "| `--port` | TCP port | 7777 |\n");
}

#[test]
fn positional_uses_angle_brackets() {
    let a = clap::Arg::new("gate_name").help("Gate name");
    assert_eq!(arg_row(&a), "| `<gate_name>` | Gate name |  |\n");
}

#[test]
fn pipe_in_help_is_escaped() {
    let a = clap::Arg::new("status").long("status").help("todo|done");
    assert!(arg_row(&a).contains("todo\\|done"));
}
