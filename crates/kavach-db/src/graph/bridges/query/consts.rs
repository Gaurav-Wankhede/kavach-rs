pub(super) const QUERIES_CONCEPTS_FOR_PROJECT: &[(&str, &str, &str)] = &[
    (
        "roadmap",
        "implements",
        "SELECT entry_key, ->implements->entity.* AS concepts FROM roadmap WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "roadmap",
        "discusses",
        "SELECT entry_key, ->discusses->entity.* AS concepts FROM roadmap WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "roadmap",
        "references_concept",
        "SELECT entry_key, ->references_concept->entity.* AS concepts FROM roadmap WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "roadmap",
        "violates",
        "SELECT entry_key, ->violates->entity.* AS concepts FROM roadmap WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "decision",
        "implements",
        "SELECT entry_key, ->implements->entity.* AS concepts FROM decision WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "decision",
        "discusses",
        "SELECT entry_key, ->discusses->entity.* AS concepts FROM decision WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "decision",
        "references_concept",
        "SELECT entry_key, ->references_concept->entity.* AS concepts FROM decision WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "decision",
        "violates",
        "SELECT entry_key, ->violates->entity.* AS concepts FROM decision WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "research",
        "implements",
        "SELECT entry_key, ->implements->entity.* AS concepts FROM research WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "research",
        "discusses",
        "SELECT entry_key, ->discusses->entity.* AS concepts FROM research WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "research",
        "references_concept",
        "SELECT entry_key, ->references_concept->entity.* AS concepts FROM research WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "research",
        "violates",
        "SELECT entry_key, ->violates->entity.* AS concepts FROM research WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "pattern",
        "implements",
        "SELECT entry_key, ->implements->entity.* AS concepts FROM pattern WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "pattern",
        "discusses",
        "SELECT entry_key, ->discusses->entity.* AS concepts FROM pattern WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "pattern",
        "references_concept",
        "SELECT entry_key, ->references_concept->entity.* AS concepts FROM pattern WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "pattern",
        "violates",
        "SELECT entry_key, ->violates->entity.* AS concepts FROM pattern WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "app_spec",
        "implements",
        "SELECT entry_key, ->implements->entity.* AS concepts FROM app_spec WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "app_spec",
        "discusses",
        "SELECT entry_key, ->discusses->entity.* AS concepts FROM app_spec WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "app_spec",
        "references_concept",
        "SELECT entry_key, ->references_concept->entity.* AS concepts FROM app_spec WHERE project.slug = $slug LIMIT $limit",
    ),
    (
        "app_spec",
        "violates",
        "SELECT entry_key, ->violates->entity.* AS concepts FROM app_spec WHERE project.slug = $slug LIMIT $limit",
    ),
];

pub(super) const QUERIES_PROJECTS_FOR_CONCEPT: &[(&str, &str)] = &[
    (
        "implements",
        "SELECT VALUE <-implements<-(roadmap, decision, research, pattern, app_spec).{ table: meta::tb(id), key: entry_key, slug: project.slug } FROM entity WHERE entity_type = 'concept' AND name = $name LIMIT $limit",
    ),
    (
        "discusses",
        "SELECT VALUE <-discusses<-(roadmap, decision, research, pattern, app_spec).{ table: meta::tb(id), key: entry_key, slug: project.slug } FROM entity WHERE entity_type = 'concept' AND name = $name LIMIT $limit",
    ),
    (
        "references_concept",
        "SELECT VALUE <-references_concept<-(roadmap, decision, research, pattern, app_spec).{ table: meta::tb(id), key: entry_key, slug: project.slug } FROM entity WHERE entity_type = 'concept' AND name = $name LIMIT $limit",
    ),
    (
        "violates",
        "SELECT VALUE <-violates<-(roadmap, decision, research, pattern, app_spec).{ table: meta::tb(id), key: entry_key, slug: project.slug } FROM entity WHERE entity_type = 'concept' AND name = $name LIMIT $limit",
    ),
];
