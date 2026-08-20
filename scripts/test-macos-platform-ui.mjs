import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (...parts) => fs.readFileSync(path.join(root, ...parts), "utf8");
const js = read("desktop", "dist", "app.js");
const main = read("desktop", "src-tauri", "src", "main.rs");
const platform = read("desktop", "src-tauri", "src", "platform.rs");
const agent = read("desktop", "src-tauri", "src", "agent_llm.rs");
const cargo = read("desktop", "src-tauri", "Cargo.toml");

const fixtureSource = js.slice(0, js.indexOf("\nconst $ ="));
function fixtureFor(search) {
  const context = {
    URLSearchParams,
    window: { __TAURI__: undefined, location: { search } },
  };
  vm.runInNewContext(`${fixtureSource}\nthis.fixture = mockPlatformFixture;`, context);
  return structuredClone(context.fixture);
}

assert.deepEqual(fixtureFor("?platform=macos-aarch64"), {
  platform: "macos-aarch64",
  rscript: "/Library/Frameworks/R.framework/Resources/bin/Rscript",
  logPath: "/Users/researcher/Library/Logs/Rho/startup.log",
  projectRoot: "/Users/researcher/Documents/Rho Mac 研究",
  alternateProjectRoot: "/Users/researcher/Documents/Rho Demo",
});
assert.equal(fixtureFor("?platform=unknown").platform, "windows-x86_64");
assert.doesNotMatch(js, /navigator\.(?:platform|userAgent)/);
assert.match(js, /platform: mockPlatformFixture\.platform/);
assert.match(js, /rscript: mockPlatformFixture\.rscript/);
assert.match(js, /path: mockPlatformFixture\.logPath/);
assert.match(js, /\[mockPlatformFixture\.projectRoot\]/);
assert.match(js, /let mockLastProject = mockPlatformFixture\.projectRoot/);

assert.match(platform, /DesktopPlatform::Windows => RscriptSelectionSpec[\s\S]*display_name: "Rscript\.exe"/);
assert.match(platform, /DesktopPlatform::Macos \| DesktopPlatform::Linux => RscriptSelectionSpec[\s\S]*display_name: "Rscript"/);
assert.match(platform, /picker_extension: None/);
assert.match(main, /set_title\(platform::rscript_picker_title\(\)\)/);
assert.match(main, /platform::rscript_picker_extension\(\)/);
assert.match(main, /platform::rscript_display_name\(\)/);

assert.match(cargo, /\[target\.'cfg\(target_os = "macos"\)'\.dependencies\][\s\S]*keyring = \{ version = "4\.1\.6", default-features = false, features = \["v1"\] \}/);
assert.match(agent, /cfg\(any\(windows, target_os = "macos", target_os = "linux"\)\)/);
assert.match(agent, /const SYSTEM_CREDENTIAL_STORE_LABEL: &str = "macOS Keychain"/);
assert.match(agent, /macos_native_keychain_set_get_replace_delete_and_cleanup/);
assert.match(agent, /ignore = "opt-in MAC3 smoke touches a unique disposable macOS Keychain entry"/);

console.log("macOS platform, Keychain, dialog, and deterministic mock contract checks passed.");
