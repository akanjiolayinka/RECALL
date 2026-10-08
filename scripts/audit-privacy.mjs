// Privacy audit: checks that Recall has no way to send data to the internet
// or to cloud AI services. Run before every release:
//
//   npm run audit:privacy
//
// 1. The Rust backend, as built for each desktop platform, contains no HTTP,
//    TLS or WebSocket client library.
// 2. The source code contains no cloud AI API names or endpoints.
// 3. The frontend makes no network calls (fetch, XMLHttpRequest, WebSocket).
// 4. The app window's Content Security Policy only allows talking to the
//    Recall backend.
//
// Exits with an error if any check fails.

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TARGETS = ["x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-unknown-linux-gnu"];
const NETWORK_CRATES = [
  "reqwest", "ureq", "hyper", "isahc", "curl", "attohttpc", "surf", "h2",
  "rustls", "native-tls", "openssl", "tungstenite", "tokio-tungstenite", "websocket",
];
const AI_SERVICES = [
  "openai", "anthropic", "gemini", "generativeai", "openrouter", "replicate.com", "groq",
  "api-inference.huggingface", "router.huggingface", "together.xyz", "api.cohere", "mistral.ai",
];
const FRONTEND_NETWORK_CALLS = [/\bfetch\s*\(/, /XMLHttpRequest/, /new\s+WebSocket\b/, /navigator\.sendBeacon/];
const SOURCE_DIRS = ["src", "src-tauri/src", "scripts"];
const SKIP = new Set(["scripts/audit-privacy.mjs", "scripts/download-models.mjs"]);

let failures = 0;
const pass = (message) => console.log(`  ✓ ${message}`);
const fail = (message) => {
  failures++;
  console.log(`  ✗ ${message}`);
};

function files(dir) {
  return readdirSync(join(ROOT, dir)).flatMap((name) => {
    const path = join(dir, name);
    return statSync(join(ROOT, path)).isDirectory() ? files(path) : [path.replaceAll("\\", "/")];
  });
}

console.log("1. Rust dependencies (per desktop platform)");
for (const target of TARGETS) {
  const tree = execFileSync("cargo", ["tree", "--target", target, "-e", "normal", "--prefix", "none"], {
    cwd: join(ROOT, "src-tauri"),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const crates = new Set(tree.split("\n").map((line) => line.split(" ")[0]));
  const found = NETWORK_CRATES.filter((name) => crates.has(name));
  if (found.length) fail(`${target}: network libraries present: ${found.join(", ")}`);
  else pass(`${target}: no HTTP/TLS/WebSocket client libraries`);
}

console.log("2. Cloud AI services in source code");
const sources = SOURCE_DIRS.flatMap(files).filter((f) => !SKIP.has(f) && /\.(rs|ts|tsx|js|mjs|py|json|toml)$/.test(f));
const aiHits = sources.flatMap((file) => {
  const text = readFileSync(join(ROOT, file), "utf8").toLowerCase();
  return AI_SERVICES.filter((name) => text.includes(name)).map((name) => `${file}: "${name}"`);
});
if (aiHits.length) aiHits.forEach((hit) => fail(hit));
else pass(`no cloud AI service names in ${sources.length} source files`);

console.log("3. Network calls in the frontend");
const frontend = files("src").filter((f) => /\.(ts|tsx)$/.test(f) && !f.endsWith(".test.ts"));
const callHits = frontend.flatMap((file) => {
  const text = readFileSync(join(ROOT, file), "utf8");
  return FRONTEND_NETWORK_CALLS.filter((re) => re.test(text)).map((re) => `${file}: ${re}`);
});
if (callHits.length) callHits.forEach((hit) => fail(hit));
else pass(`no fetch/XMLHttpRequest/WebSocket in ${frontend.length} frontend files`);

console.log("4. App window security policy (CSP)");
const config = JSON.parse(readFileSync(join(ROOT, "src-tauri/tauri.conf.json"), "utf8"));
const connect = (config.app?.security?.csp ?? "").split(";").map((d) => d.trim()).find((d) => d.startsWith("connect-src"));
if (connect === "connect-src ipc: http://ipc.localhost") pass(`only the Recall backend is reachable (${connect})`);
else fail(`connect-src should be "connect-src ipc: http://ipc.localhost", found: ${connect ?? "none"}`);

console.log(failures ? `\nPrivacy audit FAILED (${failures} problem${failures > 1 ? "s" : ""}).` : "\nPrivacy audit passed.");
process.exit(failures ? 1 : 0);
