//! Tests for Cloudflare Workers-specific database operation detection.

use crate::database_ops_guard::detect;

fn k(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn cf_kv_no_ttl_advisory() {
    let src = r#"export default { async fetch(req, env) { await env.KV.put("k", "v"); return new Response("ok"); } }"#;
    let r = detect("worker/src/index.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-kv-no-ttl"));
}

#[test]
fn cf_kv_with_ttl_ok() {
    let src = r#"export default { async fetch(req, env) { await env.KV.put("k", "v", { expirationTtl: 3600 }); return new Response("ok"); } }"#;
    let r = detect("worker/src/index.ts", src);
    assert!(!r.iter().any(|v| v.pattern == "cf-kv-no-ttl"));
}

#[test]
fn cf_kv_write_in_loop_blocked() {
    let src = r#"export default { async fetch(req, env) { for (const k of keys) { await env.KV.put(k, "v"); } } }"#;
    let r = detect("worker/src/index.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-kv-write-in-loop"));
}

#[test]
fn cf_d1_select_star_advisory() {
    let kw = k(&["SE", "LECT * FROM users"]);
    let src = [
        "export default { async fetch(req, env) { await env.DB.prepare(`",
        &kw,
        "`).all(); } }",
    ]
    .concat();
    let r = detect("worker/src/db.ts", &src);
    assert!(r.iter().any(|v| v.pattern == "cf-d1-select-star"));
}

#[test]
fn cf_r2_arraybuffer_blocked() {
    let src = r#"export default { async fetch(req, env) { const obj = await env.R2.get("k"); const buf = await obj.arrayBuffer(); return new Response(buf); } }"#;
    let r = detect("worker/src/r2.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-r2-arraybuffer"));
}

#[test]
fn cf_hyperdrive_rest_blocked() {
    let src = r#"export default { async fetch(req, env) { const r = await fetch("https://api.cloudflare.com/hyperdrive/query"); return r; } }"#;
    let r = detect("worker/src/hd.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-hyperdrive-rest"));
}

#[test]
fn cf_vectorize_no_topk_advisory() {
    let src = r"export default { async fetch(req, env) { const matches = await env.VECTORIZE.query(vec, {}); return Response.json(matches); } }";
    let r = detect("worker/src/vec.ts", src);
    assert!(r.iter().any(|v| v.pattern == "cf-vectorize-no-topk"));
}

#[test]
fn cf_vectorize_with_topk_ok() {
    let src = r"export default { async fetch(req, env) { const matches = await env.VECTORIZE.query(vec, { topK: 5 }); return Response.json(matches); } }";
    let r = detect("worker/src/vec.ts", src);
    assert!(!r.iter().any(|v| v.pattern == "cf-vectorize-no-topk"));
}
