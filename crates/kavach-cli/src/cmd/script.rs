use rhai::{Engine, Scope, Dynamic, Array};
use scraper::{Html, Selector};

pub(crate) fn run(file_path: Option<&str>, expr: Option<&str>) -> i32 {
    let mut engine = Engine::new();
    engine.set_max_operations(500_000);

    register_http_module(&mut engine);
    register_api_module(&mut engine);
    register_fs_module(&mut engine);

    let mut scope = Scope::new();

    if let Some(code) = expr {
        match engine.eval_with_scope::<Dynamic>(&mut scope, code) {
            Ok(val) => {
                if !val.is_unit() {
                    println!("{}", val);
                }
                0
            }
            Err(e) => {
                eprintln!("kavach script error: {}", e);
                1
            }
        }
    } else if let Some(path) = file_path {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("kavach script: cannot read file '{}': {}", path, e);
                return 2;
            }
        };
        match engine.eval_with_scope::<Dynamic>(&mut scope, &content) {
            Ok(val) => {
                if !val.is_unit() {
                    println!("{}", val);
                }
                0
            }
            Err(e) => {
                eprintln!("kavach script error: {}", e);
                1
            }
        }
    } else {
        eprintln!("kavach script: specify an expression with -e or provide a script file");
        2
    }
}

fn register_http_module(engine: &mut Engine) {
    engine.register_fn("http_get", |url: &str| -> String {
        ureq::get(url)
            .header("User-Agent", "kavach/1.3")
            .call()
            .and_then(|res| res.into_body().read_to_string())
            .unwrap_or_default()
    });

    engine.register_fn("http_get_json", |url: &str| -> Dynamic {
        let body: String = ureq::get(url)
            .header("User-Agent", "kavach/1.3")
            .call()
            .and_then(|res| res.into_body().read_to_string())
            .unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .map(|v| rhai::serde::to_dynamic(&v).unwrap_or(Dynamic::UNIT))
            .unwrap_or(Dynamic::UNIT)
    });
}

fn register_api_module(engine: &mut Engine) {
    // llms.txt & llms-full.txt discovery
    engine.register_fn("api_llms_txt", |base_url: &str| -> String {
        let clean = base_url.trim_end_matches('/');
        let candidates = [
            format!("{}/llms.txt", clean),
            format!("{}/llms-full.txt", clean),
            format!("{}/.well-known/llms.txt", clean),
        ];
        for url in candidates {
            if let Ok(res) = ureq::get(&url).header("User-Agent", "kavach/1.3").call() {
                if res.status().is_success() {
                    if let Ok(txt) = res.into_body().read_to_string() {
                        if !txt.trim().is_empty() {
                            return txt;
                        }
                    }
                }
            }
        }
        String::new()
    });

    // Sitemap route extractor
    engine.register_fn("api_sitemap", |base_url: &str| -> Array {
        let clean = base_url.trim_end_matches('/');
        let sitemap_url = format!("{}/sitemap.xml", clean);
        let mut routes = Array::new();
        if let Ok(res) = ureq::get(&sitemap_url).header("User-Agent", "kavach/1.3").call() {
            if let Ok(body) = res.into_body().read_to_string() {
                for line in body.lines() {
                    if line.contains("<loc>") && line.contains("</loc>") {
                        let start = line.find("<loc>").unwrap_or(0) + 5;
                        let end = line.find("</loc>").unwrap_or(line.len());
                        if start < end {
                            routes.push(Dynamic::from(line[start..end].trim().to_string()));
                        }
                    }
                }
            }
        }
        routes
    });

    // OpenAPI & Swagger JSON probing
    engine.register_fn("api_openapi", |base_url: &str| -> Dynamic {
        let clean = base_url.trim_end_matches('/');
        let candidates = [
            format!("{}/openapi.json", clean),
            format!("{}/swagger.json", clean),
            format!("{}/api-docs/openapi.json", clean),
            format!("{}/v1/openapi.json", clean),
        ];
        for url in candidates {
            if let Ok(res) = ureq::get(&url).header("User-Agent", "kavach/1.3").call() {
                if res.status().is_success() {
                    if let Ok(body) = res.into_body().read_to_string() {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                            return rhai::serde::to_dynamic(&v).unwrap_or(Dynamic::UNIT);
                        }
                    }
                }
            }
        }
        Dynamic::UNIT
    });

    // CSS Selector query on HTML
    engine.register_fn("html_select", |html: &str, selector_str: &str| -> Array {
        let mut matches = Array::new();
        let document = Html::parse_document(html);
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                matches.push(Dynamic::from(element.text().collect::<String>().trim().to_string()));
            }
        }
        matches
    });
}

fn register_fs_module(engine: &mut Engine) {
    engine.register_fn("fs_read", |path: &str| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    });

    engine.register_fn("fs_lines", |path: &str| -> i64 {
        std::fs::read_to_string(path)
            .map(|c| c.lines().count() as i64)
            .unwrap_or(0)
    });

    engine.register_fn("fs_exists", |path: &str| -> bool {
        std::path::Path::new(path).exists()
    });

    engine.register_fn("fs_write", |path: &str, content: &str| -> bool {
        std::fs::write(path, content).is_ok()
    });

    engine.register_fn("hash_blake3", |data: &str| -> String {
        blake3::hash(data.as_bytes()).to_hex().to_string()
    });
}
