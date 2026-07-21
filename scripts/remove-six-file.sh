#!/bin/bash
# Remove six-file-context system from kavach source
set -e
cd /Users/gauravwankhede/kavach-rs

# 1. Fix gate_runner.rs - remove six_file_gates module and dispatch
sd 'mod six_file_gates;\n' '' crates/kavach-engine/src/gate_runner.rs
sd '        .or_else(|| six_file_gates::dispatch(gate_name, input))\n' '' crates/kavach-engine/src/gate_runner.rs
sd 'three' 'two' crates/kavach-engine/src/gate_runner.rs
sd ', six-file' '' crates/kavach-engine/src/gate_runner.rs

# 2. Remove six_file from gates.rs
sd 'pub mod six_file;\n' '' crates/kavach-engine/src/gates.rs

# 3. Fix kavach-types/src/lib.rs - remove six_file module and exports
sd 'pub mod six_file;\n' '' crates/kavach-types/src/lib.rs
sd 'pub use six_file::\{[^}]+\};\n' '' crates/kavach-types/src/lib.rs

# 4. Remove six-file gates from gates/info.rs
sd '        "six-file-intent" => \{\n            "Six-file context: classify user intent against app_spec / roadmap scope"\n        \},\n' '' crates/kavach-cli/src/cmd/gates/info.rs
sd '        "pre-implementation" => \{\n            "Six-file context: block IMPLEMENT until unit spec \+ dependencies are loaded"\n        \},\n' '' crates/kavach-cli/src/cmd/gates/info.rs
sd '        "post-implementation" => \{\n            "Six-file context: verify implementation against unit spec before marking done"\n        \},\n' '' crates/kavach-cli/src/cmd/gates/info.rs
sd ' six-file-intent, pre-implementation, post-implementation' '' crates/kavach-cli/src/cmd/gates/info.rs
sd 'kanban inject, mistake patterns, six-file context' 'kanban inject, mistake patterns' crates/kavach-cli/src/cmd/gates/info.rs

# 5. Remove Spec command from cli.rs
sd 'mod spec;\n' '' crates/kavach-cli/src/cli.rs
sd 'pub\(crate\) use spec::SpecAction;\n' '' crates/kavach-cli/src/cli.rs
sd '    /// Manage specification artifacts \(six-file context, auto-draft\)\n    Spec \{\n        #\[command\(subcommand\)\]\n        action: SpecAction,\n    \},\n' '' crates/kavach-cli/src/cli.rs
sd 'six-file-intent, pre-implementation, post-implementation, ' '' crates/kavach-cli/src/cli.rs

# 6. Remove spec dispatch from cmd.rs
sd 'mod spec;\n' '' crates/kavach-cli/src/cmd.rs
sd '        Commands::Spec \{ action \} => spec::run\(action\),\n' '' crates/kavach-cli/src/cmd.rs

# 7. Fix session_start/lld.rs
sd 'intent\|six-file-intent' 'intent' crates/kavach-engine/src/gates/session_start/lld.rs
sd 'impl\(pre-implementation\|post-implementation\)' 'impl' crates/kavach-engine/src/gates/session_start/lld.rs

echo "Six-file removal complete"
