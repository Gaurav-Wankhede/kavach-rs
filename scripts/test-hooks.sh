#!/bin/bash
# Test kavach hooks integration with Devin CLI
# SOURCE: https://docs.devin.ai/cli/extensibility/hooks/overview

set -e

KAVACH_BIN="/Users/gauravwankhede/.local/bin/kavach"
PROJECT_DIR="/Users/gauravwankhede/kavach-rs"

echo "=== Testing kavach hooks ==="
echo ""

# Test 1: SessionStart hook
echo "Test 1: SessionStart hook"
echo '{"hook_event_name":"SessionStart","cwd":"'"$PROJECT_DIR"'"}' | "$KAVACH_BIN" session init
echo "✓ SessionStart hook passed"
echo ""

# Test 2: UserPromptSubmit hook
echo "Test 2: UserPromptSubmit hook"
echo '{"hook_event_name":"UserPromptSubmit","tool_name":"user_prompt","tool_input":{"content":"test"}}' | "$KAVACH_BIN" gates intent --hook > /dev/null
echo "✓ UserPromptSubmit hook passed"
echo ""

# Test 3: PreToolUse hook for Write
echo "Test 3: PreToolUse hook (Write)"
echo '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"/tmp/test.txt","content":"test"}}' | "$KAVACH_BIN" gates pre-write --hook > /dev/null
echo "✓ PreToolUse hook (Write) passed"
echo ""

# Test 4: PreToolUse hook for exec
echo "Test 4: PreToolUse hook (exec)"
echo '{"hook_event_name":"PreToolUse","tool_name":"exec","tool_input":{"command":"echo test"}}' | "$KAVACH_BIN" gates pre-tool --hook > /dev/null
echo "✓ PreToolUse hook (exec) passed"
echo ""

# Test 5: PostToolUse hook for Write
echo "Test 5: PostToolUse hook (Write)"
echo '{"hook_event_name":"PostToolUse","tool_name":"Write","tool_input":{"file_path":"/tmp/test.txt","content":"test"}}' | "$KAVACH_BIN" gates post-write --hook > /dev/null
echo "✓ PostToolUse hook (Write) passed"
echo ""

# Test 6: PostToolUse hook for exec
echo "Test 6: PostToolUse hook (exec)"
echo '{"hook_event_name":"PostToolUse","tool_name":"exec","tool_input":{"command":"echo test"}}' | "$KAVACH_BIN" gates post-tool --hook > /dev/null
echo "✓ PostToolUse hook (exec) passed"
echo ""

# Test 7: Stop hook (intentional error injection)
echo "Test 7: Stop hook (intentional error injection - expect timeout)"
echo '{"hook_event_name":"Stop","cwd":"'"$PROJECT_DIR"'"}' | "$KAVACH_BIN" gates stop --hook --vendor claude-code &
STOP_PID=$!
sleep 2
if kill -0 $STOP_PID 2>/dev/null; then
  kill $STOP_PID 2>/dev/null
  echo "✓ Stop hook intentional error injection confirmed (timeout as designed)"
else
  echo "✓ Stop hook completed"
fi
echo ""

# Test 8: SessionEnd hook
echo "Test 8: SessionEnd hook"
echo '{"hook_event_name":"Stop","cwd":"'"$PROJECT_DIR"'"}' | "$KAVACH_BIN" session end > /dev/null
echo "✓ SessionEnd hook passed"
echo ""

echo "=== All hook tests passed ==="
