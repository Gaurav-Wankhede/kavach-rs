#!/bin/bash
# Remove six-file-context system from kavach source
set -e
cd /Users/gauravwankhede/kavach-rs

# 1. Fix gate_runner.rs - remove six_file_gates module and dispatch
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    lines = f.readlines()

with open(sys.argv[1], "w") as f:
    for line in lines:
        if "mod six_file_gates;" in line:
            continue
        if "six_file_gates::dispatch" in line:
            continue
        if "three" in line and "gate families" in line:
            line = line.replace("three", "two")
        if "six-file" in line:
            line = line.replace(", six-file", "")
        f.write(line)
' crates/kavach-engine/src/gate_runner.rs

# 2. Remove six_file from gates.rs
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    lines = f.readlines()

with open(sys.argv[1], "w") as f:
    for line in lines:
        if "pub mod six_file;" in line:
            continue
        f.write(line)
' crates/kavach-engine/src/gates.rs

# 3. Remove six_file from kavach-types/src/lib.rs
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    lines = f.readlines()

with open(sys.argv[1], "w") as f:
    skip = False
    for line in lines:
        if "pub mod six_file;" in line:
            continue
        if "pub use six_file" in line:
            continue
        f.write(line)
' crates/kavach-types/src/lib.rs

# 4. Remove six-file gates from gates/info.rs
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    content = f.read()

# Remove six-file gate match arms
content = content.replace(
        "six-file-intent" => {
            "Six-file context: classify user intent against app_spec / roadmap scope"
        },
    "", 1)
content = content.replace(
        "pre-implementation" => {
            "Six-file context: block IMPLEMENT until unit spec + dependencies are loaded"
        },
    "", 1)
content = content.replace(
        "post-implementation" => {
            "Six-file context: verify implementation against unit spec before marking done"
        },
    "", 1)
content = content.replace(" six-file-intent, pre-implementation, post-implementation", "")

with open(sys.argv[1], "w") as f:
    f.write(content)
' crates/kavach-cli/src/cmd/gates/info.rs

# 5. Remove spec module from cli.rs and cmd.rs
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    lines = f.readlines()

with open(sys.argv[1], "w") as f:
    for line in lines:
        if "mod spec;" in line and "cli/spec.rs" not in line:
            continue
        if "pub(crate) use spec::SpecAction;" in line:
            continue
        if "Spec {" in line and "action: SpecAction" in lines[lines.index(line)+1] if lines.index(line)+1 < len(lines) else False:
            continue
        f.write(line)
' crates/kavach-cli/src/cli.rs

python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    lines = f.readlines()

with open(sys.argv[1], "w") as f:
    for line in lines:
        if line.strip() == "mod spec;":
            continue
        if "Commands::Spec { action } => spec::run(action)," in line:
            continue
        f.write(line)
' crates/kavach-cli/src/cmd.rs

# 6. Fix session_start/lld.rs
python3 -c '
import sys
with open(sys.argv[1], "r") as f:
    content = f.read()

content = content.replace("intent|six-file-intent", "intent")
content = content.replace("impl(pre-implementation|post-implementation)", "impl")

with open(sys.argv[1], "w") as f:
    f.write(content)
' crates/kavach-engine/src/gates/session_start/lld.rs

echo "Six-file removal complete"
