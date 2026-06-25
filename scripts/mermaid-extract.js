const [, , src, out] = process.argv;
const text = await Bun.file(src).text();
const blocks = [];
const push = (s) => { const t = s.trim(); if (t) blocks.push(t); };
for (const m of text.matchAll(/<pre[^>]*class=["'][^"']*\bmermaid\b[^"']*["'][^>]*>([\s\S]*?)<\/pre>/gi)) push(m[1]);
for (const m of text.matchAll(/```mermaid\r?\n([\s\S]*?)```/gi)) push(m[1]);
for (const m of text.matchAll(/:::mermaid\r?\n([\s\S]*?):::/gi)) push(m[1]);
const decode = (s) => s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/&quot;/g, '"').replace(/&#39;/g, "'");
await Bun.write(out, blocks.map((b) => "```mermaid\n" + decode(b) + "\n```").join("\n\n"));
console.log(blocks.length); // kavach:intentional — stdout IS the count contract read by mermaid-check.sh
