// Bridge-query SQL templates. One query per (table, edge) pair for
// `concepts_for_project`; one per edge for `projects_for_concept`.
// Positional params: concepts_for_project = (?1 category, ?2 edge, ?3 project, ?4 limit);
// projects_for_concept = (?1 concept name, ?2 edge, ?3 limit).

pub(super) const QUERIES_CONCEPTS_FOR_PROJECT: &[(&str, &str, &str)] = &[
    (
        "roadmap",
        "implements",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "roadmap",
        "discusses",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "roadmap",
        "references_concept",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "roadmap",
        "violates",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "decision",
        "implements",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "decision",
        "discusses",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "decision",
        "references_concept",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "decision",
        "violates",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "research",
        "implements",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "research",
        "discusses",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "research",
        "references_concept",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "research",
        "violates",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "pattern",
        "implements",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "pattern",
        "discusses",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "pattern",
        "references_concept",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "pattern",
        "violates",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "app_spec",
        "implements",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "app_spec",
        "discusses",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "app_spec",
        "references_concept",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
    (
        "app_spec",
        "violates",
        "SELECT m.entry_key, e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
         FROM memories m \
         JOIN entity_edges ed ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
           AND ed.rel_type = ?2 \
         JOIN entities e ON e.id = ed.to_id \
         WHERE m.category = ?1 AND m.project = ?3 LIMIT ?4",
    ),
];

pub(super) const QUERIES_PROJECTS_FOR_CONCEPT: &[(&str, &str)] = &[
    (
        "implements",
        "SELECT m.category, m.entry_key, m.project \
         FROM entity_edges ed \
         JOIN entities e ON e.id = ed.to_id \
         JOIN memories m ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
         WHERE e.entity_type = 'concept' AND e.name = ?1 AND ed.rel_type = ?2 \
         LIMIT ?3",
    ),
    (
        "discusses",
        "SELECT m.category, m.entry_key, m.project \
         FROM entity_edges ed \
         JOIN entities e ON e.id = ed.to_id \
         JOIN memories m ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
         WHERE e.entity_type = 'concept' AND e.name = ?1 AND ed.rel_type = ?2 \
         LIMIT ?3",
    ),
    (
        "references_concept",
        "SELECT m.category, m.entry_key, m.project \
         FROM entity_edges ed \
         JOIN entities e ON e.id = ed.to_id \
         JOIN memories m ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
         WHERE e.entity_type = 'concept' AND e.name = ?1 AND ed.rel_type = ?2 \
         LIMIT ?3",
    ),
    (
        "violates",
        "SELECT m.category, m.entry_key, m.project \
         FROM entity_edges ed \
         JOIN entities e ON e.id = ed.to_id \
         JOIN memories m ON ed.from_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
         WHERE e.entity_type = 'concept' AND e.name = ?1 AND ed.rel_type = ?2 \
         LIMIT ?3",
    ),
];
