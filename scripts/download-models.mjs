// Download Recall's local AI model files into models/ and verify them.
//
//   npm run download-models
//
// Every file is checked against the SHA-256 checksum recorded when it was
// verified (see docs/MODELS.md). A file that doesn't match is deleted, never
// used. Already-downloaded files that match are skipped.
//
// Requires Node.js 18 or newer (for the built-in fetch). No other packages.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const MODELS_DIR = join(ROOT, "models");

/**
 * Models to install. Each file lists download addresses in order of
 * preference; the ocrs host has been seen returning "not found" from one
 * address while the other worked, so both are tried.
 */
const BGE_REVISION = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"; // also in .github/workflows/build.yml
const BGE_URL = `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/${BGE_REVISION}`;

const MODELS = [
  {
    name: "ocrs OCR models (text in images)",
    files: [
      {
        path: "ocrs/text-detection.onnx",
        sha256: "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
        urls: [
          "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx",
          "https://ocrs-models.s3.amazonaws.com/text-detection.onnx",
        ],
      },
      {
        path: "ocrs/text-recognition.onnx",
        sha256: "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
        urls: [
          "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx",
          "https://ocrs-models.s3.amazonaws.com/text-recognition.onnx",
        ],
      },
    ],
  },
  {
    name: "BAAI/bge-small-en-v1.5 (meaning-based search)",
    files: [
      {
        path: "bge-small-en-v1.5/model.onnx",
        sha256: "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35",
        urls: [`${BGE_URL}/onnx/model.onnx`],
      },
      {
        path: "bge-small-en-v1.5/tokenizer.json",
        sha256: "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
        urls: [`${BGE_URL}/tokenizer.json`],
      },
    ],
  },
];

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function download(urls) {
  const problems = [];
  for (const url of urls) {
    try {
      const response = await fetch(url);
      if (response.ok) return Buffer.from(await response.arrayBuffer());
      problems.push(`${url} answered ${response.status} ${response.statusText}`);
    } catch (error) {
      problems.push(`${url} could not be reached (${error.cause?.code ?? error.message})`);
    }
  }
  throw new Error(problems.join("\n    "));
}

async function installFile(file) {
  const target = join(MODELS_DIR, file.path);
  const shown = relative(ROOT, target);
  if (existsSync(target) && sha256(readFileSync(target)) === file.sha256) {
    console.log(`  ✓ ${shown} (already installed)`);
    return true;
  }
  process.stdout.write(`  … downloading ${shown}\n`);
  let bytes;
  try {
    bytes = await download(file.urls);
  } catch (error) {
    console.error(`  ✗ ${shown}: download failed.\n    ${error.message}`);
    console.error("    Check your internet connection and try again.");
    return false;
  }
  const actual = sha256(bytes);
  if (actual !== file.sha256) {
    console.error(`  ✗ ${shown}: the downloaded file is not the expected one, so it was not saved.`);
    console.error(`    expected SHA-256 ${file.sha256}\n    got      SHA-256 ${actual}`);
    return false;
  }
  mkdirSync(dirname(target), { recursive: true });
  // Write to a temporary name first so a half-written file is never used.
  writeFileSync(`${target}.partial`, bytes);
  rmSync(target, { force: true });
  renameSync(`${target}.partial`, target);
  console.log(`  ✓ ${shown} (${(bytes.length / 1024 / 1024).toFixed(1)} MB, checksum verified)`);
  return true;
}

let ok = true;
for (const model of MODELS) {
  console.log(model.name);
  for (const file of model.files) ok = (await installFile(file)) && ok;
}
console.log(ok ? "\nAll model files are installed. Restart Recall to use them." : "\nSome files could not be installed (see above).");
process.exit(ok ? 0 : 1);
