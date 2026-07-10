#!/usr/bin/env node
import { spawnSync } from "child_process"
import { existsSync } from "fs"
import { dirname, join } from "path"
import { fileURLToPath } from "url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const bin = join(root, "vendor", process.platform === "win32" ? "gburn.exe" : "gburn")

if (!existsSync(bin)) {
  console.error(
    "gburn binary not found.\n" +
      "  npm rebuild @wiktorekdev/gburn\n" +
      "  or: cargo install --git https://github.com/wiktorekdev/gburn",
  )
  process.exit(1)
}

const result = spawnSync(bin, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false,
})

if (result.error) {
  console.error(result.error.message)
  process.exit(1)
}

process.exit(result.status ?? 1)
