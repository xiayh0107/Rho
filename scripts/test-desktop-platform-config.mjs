import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repositoryRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const tauriRoot = path.join(repositoryRoot, "desktop", "src-tauri");

async function readJson(name) {
  return JSON.parse(await readFile(path.join(tauriRoot, name), "utf8"));
}

const base = await readJson("tauri.conf.json");
const windows = await readJson("tauri.windows.conf.json");
const macos = await readJson("tauri.macos.conf.json");
const arkManifest = JSON.parse(
  await readFile(path.join(repositoryRoot, "runtime", "ark.json"), "utf8"),
);

for (const key of ["targets", "resources", "icon", "windows", "macOS"]) {
  assert.equal(base.bundle[key], undefined, `base bundle must not own ${key}`);
}
assert.equal(base.bundle.license, "AGPL-3.0-only");
assert.equal(
  base.bundle.licenseFile,
  undefined,
  "AGPL notice is a bundled resource, not an installer click-through EULA",
);
assert.deepEqual(base.plugins.updater.endpoints, [
  "https://yulab-smu.top/Rho/updates/tauri/stable.json",
]);
assert.match(base.plugins.updater.pubkey, /^[A-Za-z0-9+/=]+$/);

assert.deepEqual(windows.bundle.targets, ["nsis"]);
assert.equal(windows.bundle.createUpdaterArtifacts, true);
assert.equal(
  windows.bundle.resources["../resources/WebView2Loader.dll"],
  "WebView2Loader.dll",
);
assert.equal(
  windows.bundle.resources["../resources/runtime/"],
  "resources/runtime/",
);
assert.equal(windows.bundle.resources["../../LICENSE"], "licenses/rho/LICENSE.txt");
assert.equal(
  windows.bundle.resources["../../LICENSES.md"],
  "licenses/rho/THIRD-PARTY-NOTICES.md",
);
assert.ok(windows.bundle.icon.includes("icons/icon.ico"));

assert.deepEqual(macos.bundle.targets, ["app", "dmg"]);
assert.equal(macos.bundle.createUpdaterArtifacts, true);
assert.deepEqual(macos.bundle.externalBin, ["binaries/ark"]);
assert.deepEqual(macos.bundle.resources, {
  "../../LICENSE": "licenses/rho/LICENSE.txt",
  "../../LICENSES.md": "licenses/rho/THIRD-PARTY-NOTICES.md",
  "../resources/runtime/LICENSE": "licenses/ark/LICENSE",
  "../resources/runtime/NOTICE": "licenses/ark/NOTICE",
});
assert.ok(macos.bundle.icon.includes("icons/icon.icns"));
assert.equal(macos.bundle.macOS.minimumSystemVersion, "14.0");
assert.equal(macos.bundle.macOS.hardenedRuntime, true);
assert.equal(macos.bundle.macOS.entitlements, "Entitlements.plist");

const entitlements = await readFile(path.join(tauriRoot, "Entitlements.plist"), "utf8");
assert.deepEqual(
  [...entitlements.matchAll(/<key>([^<]+)<\/key>/g)].map((match) => match[1]),
  ["com.apple.security.cs.disable-library-validation"],
  "macOS signing must use only the reviewed library-validation exception",
);
assert.match(
  entitlements,
  /<key>com\.apple\.security\.cs\.disable-library-validation<\/key>\s*<true\/>/,
);
assert.doesNotMatch(entitlements, /com\.apple\.security\.app-sandbox/);

for (const name of ["32x32.png", "128x128.png", "128x128@2x.png", "icon-512.png"]) {
  const image = await readFile(path.join(tauriRoot, "icons", name));
  assert.equal(image.subarray(1, 4).toString("ascii"), "PNG", `${name} must be PNG`);
  assert.equal(image[25], 6, `${name} must use PNG RGBA color type`);
}

const icns = await readFile(path.join(tauriRoot, "icons", "icon.icns"));
assert.equal(icns.subarray(0, 4).toString("ascii"), "icns");

assert.equal(arkManifest.version, "0.1.252");
assert.deepEqual(arkManifest["macos-arm64"], {
  url: "https://github.com/posit-dev/ark/releases/download/0.1.252/ark-0.1.252-darwin-arm64.zip",
  sha256: "aa1186f6e1ad271abaf246fd76e0aa9039cdeeff2cb52147e8887060afd5fb07",
});

const gitignore = await readFile(path.join(repositoryRoot, ".gitignore"), "utf8");
assert.match(gitignore, /^\/desktop\/src-tauri\/binaries\/ark-\*$/m);

console.log("desktop platform configuration is valid");
