import { readFileSync, writeFileSync, chmodSync } from "fs"

const path = "dist/cli.js"
let src = readFileSync(path, "utf8")
// drop any existing shebangs (bun may preserve source shebang)
src = src.replace(/^(#!.*\r?\n)+/, "")
writeFileSync(path, "#!/usr/bin/env node\n" + src)
try {
  chmodSync(path, 0o755)
} catch {
  /* windows ok */
}
