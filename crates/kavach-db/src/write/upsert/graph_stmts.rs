// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)

/// Append entity + project + reference RELATE statements to the txn buffer.
pub(super) fn append_entity_graph_stmts(
    q: &mut String,
    qualified_name: &str,
    references: &[String],
) {
    if qualified_name.is_empty() {
        return;
    }
    q.push_str(
        "LET $entry_node = (SELECT VALUE id FROM entity \
            WHERE entity_type = 'memory' AND name = $qname LIMIT 1)[0] \
            ?? (CREATE type::record('entity', string::concat('memory:', $qname)) \
                SET entity_type = 'memory', name = $qname, \
                updated_at = time::now() RETURN id).id;\n",
    );
    q.push_str(
        "LET $project_node = (SELECT VALUE id FROM entity \
            WHERE entity_type = 'project' AND name = $project_name LIMIT 1)[0] \
            ?? (CREATE type::record('entity', string::concat('project:', $project_name)) \
                SET entity_type = 'project', name = $project_name, \
                updated_at = time::now() RETURN id).id;\n",
    );
    q.push_str("RELATE $entry_node->in_scope->$project_node SET weight = 1.0;\n");
    if !references.is_empty() {
        q.push_str(
            "FOR $ref IN $refs { \
                LET $skill_node = (SELECT VALUE id FROM entity \
                    WHERE entity_type = 'skill' AND name = $ref LIMIT 1)[0] \
                    ?? (CREATE type::record('entity', string::concat('skill:', $ref)) \
                        SET entity_type = 'skill', name = $ref, \
                        updated_at = time::now() RETURN id).id; \
                RELATE $entry_node->references->$skill_node SET weight = 1.0; \
            };\n",
        );
    }
}
