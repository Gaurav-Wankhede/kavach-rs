schema: kavach-agent-pointer-v1
kind: RootRouterPointer
id: kavach-pointer
version: "1.0"
metadata:
  harness: project-root-pointer
  canonical_yaml: AGENTS.yaml
  query_command: "dasel query -i yaml --var doc=yaml:file:AGENTS.yaml '$doc.<KEY>'"
graph_nodes:
  root: AGENTS.yaml
governance_notice:
  global_rules: "Enforced via RULE[user_global]. Specifications reside in AGENTS.yaml."
