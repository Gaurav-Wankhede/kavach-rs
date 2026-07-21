#!/bin/bash
# Remove six-file-context system from kavach source
# This script makes all necessary changes atomically

set -e

cd /Users/gauravwankhede/kavach-rs

# 1. Remove six_file module from gates.rs
sd 'pub mod six_file;' '' crates/kavach-engine/src/gates.rs

# 2. Remove six_file_gates from gate_runner.rs
sd 'mod six_file_gates;' '' crates/kavach-engine/src/gate_runner.rs
sd '        .or_else(|| six_file_gates::dispatch(gate_name, input))' '' crates/kavach-engine/src/gate_runner.rs
sd 'three' 'two' crates/kavach-engine/src/gate_runner.rs
sd ', six-file' '' crates/kavach-engine/src/gate_runner.rs

# 3. Remove six_file types from kavach-types/src/lib.rs
sd 'pub mod six_file;' '' crates/kavach-types/src/lib.rs
sd 'pub use six_file::\{[^}]+\};' '' crates/kavach-types/src/lib.rs

# 4. Remove six-file gate descriptions from gates/info.rs
sd '        "six-file-intent" => \{[^}]+\},' '' crates/kavach-cli/src/cmd/gates/info.rs
sd '        "pre-implementation" => \{[^}]+\},' '' crates/kavach-cli/src/cmd/gates/info.rs
sd '        "post-implementation" => \{[^}]+\},' '' crates/kavach-cli/src/cmd/gates/info.rs
sd ' six-file-intent, pre-implementation, post-implementation' '' crates/kavach-cli/src/cmd/gates/info.rs

# 5. Remove Spec command from CLI
sd 'mod spec;' '' crates/kavach-cli/src/cli.rs
sd 'pub\(crate\) use spec::SpecAction;' '' crates/kavach-cli/src/cli.rs
sd '    Spec \{[^}]+\},' '' crates/kavach-cli/src/cli.rs
sd '        Commands::Spec \{ action \} => spec::run\(action\),' '' crates/kavach-cli/src/cmd.rs
sd 'mod spec;' '' crates/kavach-cli/src/cmd.rs

# 6. Remove six-file from session_start/lld.rs
sd 'intent\|six-file-intent' 'intent' crates/kavach-engine/src/gates/session_start/lld.rs
sd 'impl\(pre-implementation\|post-implementation\)' 'impl' crates/kavach-engine/src/gates/session_start/lld.rs

echo "Six-file removal complete"
