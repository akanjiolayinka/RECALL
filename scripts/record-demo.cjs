// Records docs/assets/recall-demo.gif: a short tour of Recall's interface in
// mock mode (`npm run dev:mock`), driven by a headless browser.
//
// The GIF shows the real interface with the made-up data from
// src/lib/api/mock.ts. Mock search only matches words in file names, so the
// GIF shows the interface, not the quality of Recall's search. Re-run this
// whenever the interface changes:
//
//   npm install --no-save playwright   # not a project dependency on purpose
//   npx playwright install chromium
//   node scripts/record-demo.cjs
//
// Also needs ffmpeg on your PATH. It starts the mock-mode dev server itself
// unless something is already serving http://localhost:1420.

const { execFileSync, spawn } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const ROOT = path.join(__dirname, "..");
const OUTPUT = path.join(ROOT, "docs", "assets", "recall-demo.gif");
const URL = "http://localhost:1420";
const VIEWPORT = { width: 1100, height: 690 };
const GIF_WIDTH = 960;
const FPS = 15;

let chromium;
try {
  ({ chromium } = require("playwright"));
} catch {
  console.error("This script needs Playwright: npm install --no-save playwright && npx playwright install chromium");
  process.exit(1);
}

async function isServing() {
  try {
    return (await fetch(URL)).ok;
  } catch {
    return false;
  }
}

async function startMockServer() {
  if (await isServing()) return null;
  // The same as `npm run dev:mock`, but started directly so that stopping it
  // doesn't leave a Vite process behind.
  const vite = path.join(ROOT, "node_modules", "vite", "bin", "vite.js");
  const server = spawn(process.execPath, [vite, "--mode", "mock"], { cwd: ROOT, stdio: "ignore" });
  for (let i = 0; i < 120; i++) {
    if (await isServing()) return server;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  server.kill();
  throw new Error(`the mock-mode dev server didn't start serving ${URL}`);
}

// Headless screenshots have no mouse pointer, so draw one, or viewers can't
// tell what was clicked.
const POINTER_SCRIPT = `
  const pointer = document.createElement("div");
  pointer.id = "demo-pointer";
  Object.assign(pointer.style, {
    position: "fixed", left: "0px", top: "0px", width: "22px", height: "22px",
    margin: "-11px 0 0 -11px", borderRadius: "50%", pointerEvents: "none",
    zIndex: "2147483647", background: "rgba(37, 99, 235, 0.35)",
    border: "2px solid rgba(37, 99, 235, 0.9)", transition: "transform 120ms",
  });
  document.body.appendChild(pointer);
`;

async function main() {
  const server = await startMockServer();
  const frameDir = fs.mkdtempSync(path.join(os.tmpdir(), "recall-demo-"));
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: VIEWPORT, colorScheme: "light" });
  const frames = [];
  let pointerAt = { x: VIEWPORT.width / 2, y: VIEWPORT.height / 2 };

  const shot = async (seconds) => {
    const file = path.join(frameDir, `${String(frames.length).padStart(4, "0")}.png`);
    await page.screenshot({ path: file });
    frames.push({ file, seconds });
  };
  const placePointer = (x, y, scale = 1) =>
    page.evaluate(
      ([x, y, scale]) => {
        const pointer = document.getElementById("demo-pointer");
        pointer.style.left = `${x}px`;
        pointer.style.top = `${y}px`;
        pointer.style.transform = `scale(${scale})`;
      },
      [x, y, scale],
    );
  const moveTo = async (locator) => {
    const box = await locator.boundingBox();
    const target = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    const steps = 10;
    for (let i = 1; i <= steps; i++) {
      const t = i / steps;
      const eased = t * t * (3 - 2 * t);
      await placePointer(
        pointerAt.x + (target.x - pointerAt.x) * eased,
        pointerAt.y + (target.y - pointerAt.y) * eased,
      );
      await shot(0.04);
    }
    pointerAt = target;
  };
  const click = async (locator) => {
    await moveTo(locator);
    await placePointer(pointerAt.x, pointerAt.y, 0.7);
    await shot(0.15);
    await placePointer(pointerAt.x, pointerAt.y, 1);
    await locator.click();
  };

  try {
    await page.goto(URL);
    await page.getByText("What are you trying to remember?").waitFor();
    await page.evaluate(POINTER_SCRIPT);
    await placePointer(pointerAt.x, pointerAt.y);
    await shot(1.8);

    // Add a folder and watch it being indexed.
    await click(page.getByRole("button", { name: "Library" }));
    await page.getByRole("heading", { name: "Library" }).waitFor();
    await shot(0.8);
    await click(page.getByRole("button", { name: "Add folder" }).first());
    const done = page.getByText(/5 files · 5 read/);
    for (let i = 0; i < 60 && !(await done.isVisible()); i++) {
      await shot(0.15);
      await page.waitForTimeout(150);
    }
    await shot(1.8);

    // Search for something remembered.
    await click(page.getByRole("button", { name: "Search" }).first());
    const box = page.getByRole("searchbox").or(page.locator('form[role="search"] input')).first();
    await box.waitFor();
    await click(box);
    for (const character of "where is my lease") {
      await page.keyboard.type(character);
      await shot(0.07);
    }
    await shot(0.4);
    await page.keyboard.press("Enter");
    await page.getByRole("button", { name: /Show source/ }).first().waitFor();
    await page.waitForTimeout(300);
    await shot(2.2);

    // Open the passage it found.
    await click(page.getByRole("button", { name: /Show source/ }).first());
    await page.getByRole("dialog").waitFor();
    await page.waitForTimeout(300);
    await shot(3.2);
    await page.keyboard.press("Escape");
    await page.waitForTimeout(300);
    await shot(1.2);
  } finally {
    await browser.close();
    server?.kill();
  }

  // Each frame is held for its duration; ffmpeg builds one palette for the
  // whole GIF so the colours stay steady.
  const list = path.join(frameDir, "frames.txt");
  const lines = frames.flatMap(({ file, seconds }) => [`file '${file}'`, `duration ${seconds}`]);
  lines.push(`file '${frames[frames.length - 1].file}'`);
  fs.writeFileSync(list, lines.join("\n"));
  fs.mkdirSync(path.dirname(OUTPUT), { recursive: true });
  execFileSync("ffmpeg", [
    "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", list,
    "-vf", `fps=${FPS},scale=${GIF_WIDTH}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`,
    "-loop", "0", OUTPUT,
  ]);
  fs.rmSync(frameDir, { recursive: true, force: true });
  const megabytes = fs.statSync(OUTPUT).size / 1024 / 1024;
  console.log(`Wrote ${path.relative(ROOT, OUTPUT)} (${frames.length} frames, ${megabytes.toFixed(1)} MB)`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
