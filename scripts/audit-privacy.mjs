// Privacy audit: checks that Recall has no way to send data to the internet
// or to cloud AI services. Run before every release:
//
//   npm run audit:privacy
//
// 1. The Rust backend, as built for each desktop platform, contains no HTTP,
//    TLS or WebSocket client, updater or telemetry library, and Recall's own
//    Rust code opens no sockets.
// 2. The source code contains no cloud AI API names or endpoints.
// 3. The frontend makes no network calls (fetch, XMLHttpRequest, WebSocket,
//    EventSource, beacons), contains no web addresses, and its npm
//    dependencies include no HTTP, updater or analytics packages.
// 4. The app window's Content Security Policy only allows talking to the
//    Recall backend, and the window has no network-related permissions.
//
// The one-time model download (scripts/download-models.mjs) is the only
// part of the project that is meant to use the internet, and it runs
// outside the app.
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
  "tauri-plugin-http", "tauri-plugin-websocket", "tauri-plugin-updater", "sentry", "posthog-rs",
];
// Opening a socket directly needs none of the crates above.
const RUST_SOCKET_APIS = [/\bstd::net\b/, /\bTcpStream\b/, /\bTcpListener\b/, /\bUdpSocket\b/];
const AI_SERVICES = [
  "openai", "anthropic", "gemini", "generativeai", "openrouter", "replicate.com", "groq",
  "api-inference.huggingface", "router.huggingface", "together.xyz", "api.cohere", "mistral.ai",
];
const FRONTEND_NETWORK_CALLS = [
  /\bfetch\s*\(/, /XMLHttpRequest/, /new\s+WebSocket\b/, /new\s+EventSource\b/, /navigator\.sendBeacon/,
  /https?:\/\/(?!ipc\.localhost)/,
];
const NETWORK_NPM_PACKAGES = [
  "@tauri-apps/plugin-http", "@tauri-apps/plugin-websocket", "@tauri-apps/plugin-updater",
  "axios", "socket.io-client", "ky", "got", "node-fetch", "@sentry/react", "@sentry/browser", "posthog-js",
];
// Window permissions that would let the frontend reach the internet.
const NETWORK_PERMISSIONS = /^(http|websocket|updater|opener:allow-open-url|opener:default)/;
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
  else pass(`${target}: no HTTP/TLS/WebSocket, updater or telemetry libraries`);
}
const rustSources = files("src-tauri/src").filter((f) => f.endsWith(".rs"));
const socketHits = rustSources.flatMap((file) => {
  const text = readFileSync(join(ROOT, file), "utf8");
  return RUST_SOCKET_APIS.filter((re) => re.test(text)).map((re) => `${file}: ${re}`);
});
if (socketHits.length) socketHits.forEach((hit) => fail(`opens a socket: ${hit}`));
else pass(`no socket code in ${rustSources.length} Rust files`);

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
else pass(`no network calls or web addresses in ${frontend.length} frontend files`);
const packageJson = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
const npmDeps = Object.keys({ ...packageJson.dependencies, ...packageJson.devDependencies });
const npmHits = NETWORK_NPM_PACKAGES.filter((name) => npmDeps.includes(name));
if (npmHits.length) fail(`network or analytics npm packages: ${npmHits.join(", ")}`);
else pass(`no HTTP, updater or analytics packages among ${npmDeps.length} npm dependencies`);

console.log("4. App window security policy (CSP) and permissions");
const config = JSON.parse(readFileSync(join(ROOT, "src-tauri/tauri.conf.json"), "utf8"));
const connect = (config.app?.security?.csp ?? "").split(";").map((d) => d.trim()).find((d) => d.startsWith("connect-src"));
if (connect === "connect-src ipc: http://ipc.localhost") pass(`only the Recall backend is reachable (${connect})`);
else fail(`connect-src should be "connect-src ipc: http://ipc.localhost", found: ${connect ?? "none"}`);
const capabilities = files("src-tauri/capabilities").filter((f) => f.endsWith(".json"));
const permissionHits = capabilities.flatMap((file) => {
  const { permissions = [] } = JSON.parse(readFileSync(join(ROOT, file), "utf8"));
  return permissions
    .map((p) => (typeof p === "string" ? p : p.identifier))
    .filter((id) => NETWORK_PERMISSIONS.test(id))
    .map((id) => `${file}: ${id}`);
});
if (permissionHits.length) permissionHits.forEach((hit) => fail(`network permission: ${hit}`));
else pass(`no network permissions in the window's capabilities`);

console.log(failures ? `\nPrivacy audit FAILED (${failures} problem${failures > 1 ? "s" : ""}).` : "\nPrivacy audit passed.");
process.exit(failures ? 1 : 0);
