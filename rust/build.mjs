// Builds the Rust planner and packs it into a single, double-clickable index.html.
//
//   node rust/build.mjs
//
// Steps: cargo build (wasm32) -> wasm-bindgen (no-modules glue) -> inline the glue
// and the .wasm (base64) into rust/shell.html -> write ../index.html.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const here = dirname(fileURLToPath(import.meta.url));
const bin = join(homedir(), ".cargo", "bin");
const env = { ...process.env, PATH: bin + (process.platform === "win32" ? ";" : ":") + process.env.PATH };
const run = (cmd, args) => execFileSync(cmd, args, { cwd: here, stdio: "inherit", env });

run("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown"]);
mkdirSync(join(here, "pkg"), { recursive: true });
run("wasm-bindgen", [
  "--target", "no-modules", "--no-typescript", "--remove-name-section", "--remove-producers-section", "--out-dir", "pkg",
  join("target", "wasm32-unknown-unknown", "release", "mcu_planner.wasm"),
]);

// The generated loader resolves its own URL on start-up. Inlined into a page opened
// from disk (or from a data: URL) there is nothing to resolve, so make that step safe.
const urlLine = "script_src = new URL(document.currentScript.src, location.href).toString();";
let glue = readFileSync(join(here, "pkg", "mcu_planner.js"), "utf8");
if (glue.includes(urlLine)) glue = glue.replace(urlLine, () => `try { ${urlLine} } catch (e) {}`);
const wasm = readFileSync(join(here, "pkg", "mcu_planner_bg.wasm"));
if (glue.includes("</script")) throw new Error("glue contains </script>");

const boot =
  `<script>${glue}</script>\n` +
  `<script>(function(){var s=atob("${wasm.toString("base64")}"),b=new Uint8Array(s.length);` +
  `for(var i=0;i<s.length;i++)b[i]=s.charCodeAt(i);wasm_bindgen.initSync({module:b});})();</script>`;

const shell = readFileSync(join(here, "shell.html"), "utf8");
if (!shell.includes("<!--APP-->")) throw new Error("shell.html is missing <!--APP-->");
const out = shell.replace("<!--APP-->", () => boot);
writeFileSync(join(here, "..", "index.html"), out);
console.log(`index.html written: ${(out.length / 1024).toFixed(0)} KB (wasm ${(wasm.length / 1024).toFixed(0)} KB)`);
