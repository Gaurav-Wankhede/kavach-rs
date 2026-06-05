pub(crate) fn infer_file_patterns(description: &str, triggers: &[String]) -> Vec<String> {
    let combined = format!("{description} {}", triggers.join(" ")).to_lowercase();
    let mut patterns: Vec<String> = Vec::new();
    if combined.contains("rust")
        || combined.contains("cargo")
        || combined.contains(".rs")
        || combined.contains("struct")
        || combined.contains("trait")
        || combined.contains("impl")
        || combined.contains("thiserror")
        || combined.contains("tokio")
    {
        patterns.push("**/*.rs".to_owned());
    }
    if combined.contains("typescript")
        || combined.contains("react")
        || combined.contains("tsx")
        || combined.contains("jsx")
        || combined.contains("astro")
        || combined.contains("svelte")
        || combined.contains("vue")
        || combined.contains("frontend")
    {
        patterns.push("**/*.tsx".to_owned());
        patterns.push("**/*.ts".to_owned());
    }
    if combined.contains("sql")
        || combined.contains("postgresql")
        || combined.contains("migration")
        || combined.contains("database")
        || combined.contains("query")
        || combined.contains("sqlx")
    {
        patterns.push("**/*.sql".to_owned());
        patterns.push("**/*.rs".to_owned());
    }
    if combined.contains("docker")
        || combined.contains("kubernetes")
        || combined.contains("grpc")
        || combined.contains("yaml")
    {
        patterns.push("**/Dockerfile".to_owned());
        patterns.push("**/*.yaml".to_owned());
    }
    if combined.contains("css") || combined.contains("tailwind") || combined.contains("styling") {
        patterns.push("**/*.css".to_owned());
    }
    patterns
}
