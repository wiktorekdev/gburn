import { createWriteStream, chmodSync, mkdirSync, existsSync, renameSync, unlinkSync } from "fs"
import { pipeline } from "stream/promises"
import { join, dirname } from "path"
import { fileURLToPath } from "url"
import { createRequire } from "module"
import https from "https"
import http from "http"

const require = createRequire(import.meta.url)
const pkg = require("../package.json")
const version = pkg.version
const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const vendorDir = join(root, "vendor")

function assetForPlatform() {
  const { platform, arch } = process
  if (platform === "win32" && (arch === "x64" || arch === "arm64")) {
    return { name: "gburn-x86_64-pc-windows-msvc.exe", out: "gburn.exe" }
  }
  if (platform === "linux" && arch === "x64") {
    return { name: "gburn-x86_64-unknown-linux-gnu", out: "gburn" }
  }
  if (platform === "darwin" && arch === "arm64") {
    return { name: "gburn-aarch64-apple-darwin", out: "gburn" }
  }
  if (platform === "darwin" && arch === "x64") {
    return { name: "gburn-x86_64-apple-darwin", out: "gburn" }
  }
  return null
}

function fetchRedirect(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 8) {
      reject(new Error("too many redirects"))
      return
    }
    const lib = url.startsWith("https") ? https : http
    lib
      .get(url, { headers: { "User-Agent": "gburn-npm-install" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume()
          resolve(fetchRedirect(res.headers.location, redirects + 1))
          return
        }
        if (res.statusCode !== 200) {
          res.resume()
          reject(new Error(`download failed: HTTP ${res.statusCode} for ${url}`))
          return
        }
        resolve(res)
      })
      .on("error", reject)
  })
}

async function main() {
  const asset = assetForPlatform()
  if (!asset) {
    console.warn(
      `[gburn] no prebuilt binary for ${process.platform}/${process.arch}. ` +
        `Install with: cargo install --git https://github.com/wiktorekdev/gburn --tag v${version}`,
    )
    return
  }

  mkdirSync(vendorDir, { recursive: true })
  const dest = join(vendorDir, asset.out)
  const tmp = `${dest}.tmp`
  const url = `https://github.com/wiktorekdev/gburn/releases/download/v${version}/${asset.name}`

  try {
    if (existsSync(tmp)) unlinkSync(tmp)
    const res = await fetchRedirect(url)
    await pipeline(res, createWriteStream(tmp))
    if (existsSync(dest)) unlinkSync(dest)
    renameSync(tmp, dest)
    if (process.platform !== "win32") {
      chmodSync(dest, 0o755)
    }
    console.log(`[gburn] installed ${asset.name} → vendor/${asset.out}`)
  } catch (err) {
    try {
      if (existsSync(tmp)) unlinkSync(tmp)
    } catch {
      /* ignore */
    }
    console.warn(
      `[gburn] could not download binary (${err.message}). ` +
        `Install with: cargo install --git https://github.com/wiktorekdev/gburn --tag v${version}`,
    )
  }
}

main()
