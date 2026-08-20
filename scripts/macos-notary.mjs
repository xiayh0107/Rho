import crypto from "node:crypto";
import fs from "node:fs";
import https from "node:https";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";

import {
  CANDIDATE_REPOSITORY,
  REHEARSAL_REPOSITORY,
  validateCandidateIdentity,
} from "./candidate-release.mjs";

export const MAX_NOTARY_RECEIPT_BYTES = 64 * 1024;
export const MAX_NOTARY_STATUS_BYTES = 64 * 1024;
export const MAX_NOTARY_LOG_URL_BYTES = 64 * 1024;
export const MAX_NOTARY_LOG_BYTES = 1024 * 1024;
export const MAX_NOTARY_EVIDENCE_BYTES = 64 * 1024;
export const MAX_NOTARY_DMG_BYTES = 4 * 1024 * 1024 * 1024;
export const DEFAULT_POLL_INTERVAL_MS = 120_000;
export const DEFAULT_MAX_WAIT_MS = 19_800_000;
export const DEFAULT_REQUEST_TIMEOUT_MS = 30_000;
export const DEFAULT_MAX_TRANSIENT_ERRORS = 8;
export const NOTARY_API_ORIGIN = "https://appstoreconnect.apple.com";

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;
const RUN_ID_PATTERN = /^[1-9][0-9]{0,19}$/;
const RUN_ATTEMPT_PATTERN = /^[1-9][0-9]{0,5}$/;
const API_KEY_ID_PATTERN = /^[A-Z0-9]{10}$/;
const APPLE_STATUSES = new Set(["Accepted", "In Progress", "Invalid", "Rejected"]);
const EXACT_DEVELOPER_LOG_HOSTS = new Set(["notary-artifacts-prod.s3.amazonaws.com"]);

function fail(message) {
  throw new Error(message);
}

function assertObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  return value;
}

function assertExactKeys(value, keys, label) {
  assertObject(value, label);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(`${label} keys are invalid`);
  }
}

function boundedString(value, label, maxLength = 512) {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength || /[\u0000-\u001f\u007f]/.test(value)) {
    fail(`${label} is invalid`);
  }
  return value;
}

function normalizeSubmissionId(value, label = "Notary submission ID") {
  const normalized = boundedString(value, label, 36).toLowerCase();
  if (!UUID_PATTERN.test(normalized)) fail(`${label} must be a UUID`);
  return normalized;
}

function validateRunIdentity(runId, runAttempt) {
  if (!RUN_ID_PATTERN.test(String(runId))) fail("GitHub Run ID is invalid");
  if (!RUN_ATTEMPT_PATTERN.test(String(runAttempt))) fail("GitHub Run Attempt is invalid");
  return { run_id: String(runId), run_attempt: String(runAttempt) };
}

function validateRepositoryMode(repository, buildMode) {
  if (buildMode === "rehearsal" && repository === REHEARSAL_REPOSITORY) return;
  if (buildMode === "candidate" && repository === CANDIDATE_REPOSITORY) return;
  fail(`Notary build mode ${buildMode || "<empty>"} is not authorized for repository ${repository || "<empty>"}`);
}

function validateIdentity({ repository, buildMode, version, releaseTag, commit, runId, runAttempt }) {
  validateRepositoryMode(repository, buildMode);
  const candidate = validateCandidateIdentity(version, releaseTag, commit);
  const run = validateRunIdentity(runId, runAttempt);
  return {
    source_repository: repository,
    build_mode: buildMode,
    version: candidate.version,
    release_tag: candidate.release_tag,
    commit: candidate.commit,
    ...run,
  };
}

function readRegularFile(filePath, maxBytes, label) {
  const stat = fs.lstatSync(filePath);
  if (stat.isSymbolicLink() || !stat.isFile() || stat.size <= 0 || stat.size > maxBytes) {
    fail(`${label} is missing, empty, oversized, or a symlink`);
  }
  return fs.readFileSync(filePath);
}

function parseJsonBuffer(buffer, label, maxBytes = Number.MAX_SAFE_INTEGER) {
  if (!Buffer.isBuffer(buffer) || buffer.length === 0 || buffer.length > maxBytes) {
    fail(`${label} is empty or exceeds its byte budget`);
  }
  let value;
  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  } catch {
    fail(`${label} is not valid UTF-8 JSON`);
  }
  try {
    value = JSON.parse(text);
  } catch {
    fail(`${label} is not valid JSON`);
  }
  return assertObject(value, label);
}

function readJsonFile(filePath, maxBytes, label) {
  return parseJsonBuffer(readRegularFile(filePath, maxBytes, label), label);
}

function writeExclusive(filePath, bytes, maxBytes, label) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0 || bytes.length > maxBytes) fail(`${label} is empty or oversized`);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, bytes, { flag: "wx", mode: 0o600 });
}

function writeJsonExclusive(filePath, value, label) {
  const bytes = Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
  writeExclusive(filePath, bytes, MAX_NOTARY_EVIDENCE_BYTES, label);
}

function hashFile(filePath, maxBytes = MAX_NOTARY_DMG_BYTES) {
  const stat = fs.lstatSync(filePath);
  if (stat.isSymbolicLink() || !stat.isFile() || stat.size <= 0 || stat.size > maxBytes) {
    fail(`Notary artifact ${path.basename(filePath)} is missing, empty, oversized, or a symlink`);
  }
  const descriptor = fs.openSync(filePath, "r");
  const digest = crypto.createHash("sha256");
  const chunk = Buffer.allocUnsafe(1024 * 1024);
  try {
    while (true) {
      const count = fs.readSync(descriptor, chunk, 0, chunk.length, null);
      if (count === 0) break;
      digest.update(chunk.subarray(0, count));
    }
  } finally {
    fs.closeSync(descriptor);
  }
  return { size_bytes: stat.size, sha256: digest.digest("hex") };
}

function hashBuffer(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

const NOTARY_ARTIFACT_KINDS = new Set(["dmg", "app_zip"]);

function normalizedArtifactKind(value) {
  if (!NOTARY_ARTIFACT_KINDS.has(value)) fail("Notary artifact kind is invalid");
  return value;
}

function expectedNotaryArtifactName(version, artifactKind) {
  const kind = normalizedArtifactKind(artifactKind);
  return kind === "dmg"
    ? `Rho_${version}_aarch64.dmg`
    : `Rho_${version}_aarch64.app.zip`;
}

function recordArtifactKind(record) {
  return normalizedArtifactKind(record.artifact_kind ?? "dmg");
}

function expectedNotaryLogName(version, artifactKind) {
  return artifactKind === "dmg"
    ? `rho-${version}-macos-notary-log.json`
    : `rho-${version}-macos-app-archive-notary-log.json`;
}

function commonRecord(identity, status, type, artifactKind = "dmg") {
  const record = {
    schema_version: 1,
    type,
    status,
    ...identity,
    platform: "macos_aarch64",
  };
  if (normalizedArtifactKind(artifactKind) !== "dmg") record.artifact_kind = artifactKind;
  return record;
}

export function createPendingRecord({ receiptPath, dmgPath, artifactPath, artifactKind = "dmg", ...identityInput }) {
  const identity = validateIdentity(identityInput);
  const receipt = readJsonFile(receiptPath, MAX_NOTARY_RECEIPT_BYTES, "Notary submission receipt");
  const id = normalizeSubmissionId(receipt.id);
  if (receipt.message != null) boundedString(receipt.message, "Notary submission message", 1024);
  const kind = normalizedArtifactKind(artifactKind);
  const submittedArtifactPath = artifactPath ?? dmgPath;
  if (!submittedArtifactPath) fail("Notary submission artifact is missing");
  const artifactName = path.basename(submittedArtifactPath);
  if (artifactName !== expectedNotaryArtifactName(identity.version, kind)) {
    fail("Submitted notarization artifact name does not match the candidate identity");
  }
  const artifact = hashFile(submittedArtifactPath);
  return {
    ...commonRecord(identity, "pending", "rho_macos_notary_pending", kind),
    submission: {
      id,
      artifact_name: artifactName,
      size_bytes: artifact.size_bytes,
      sha256: artifact.sha256,
    },
  };
}

function validateCommonRecord(record, expectedType, expectedStatus) {
  const baseKeys = [
    "schema_version",
    "type",
    "status",
    "source_repository",
    "build_mode",
    "version",
    "release_tag",
    "commit",
    "run_id",
    "run_attempt",
    "platform",
    "submission",
  ];
  const artifactKind = recordArtifactKind(record);
  assertExactKeys(
    record,
    Object.hasOwn(record, "artifact_kind") ? [...baseKeys, "artifact_kind"] : baseKeys,
    expectedType,
  );
  if (record.schema_version !== 1 || record.type !== expectedType || record.status !== expectedStatus) {
    fail(`${expectedType} schema, type, or status is invalid`);
  }
  if (record.platform !== "macos_aarch64") fail(`${expectedType} platform is invalid`);
  validateIdentity({
    repository: record.source_repository,
    buildMode: record.build_mode,
    version: record.version,
    releaseTag: record.release_tag,
    commit: record.commit,
    runId: record.run_id,
    runAttempt: record.run_attempt,
  });
  return { record, artifact_kind: artifactKind };
}

function validateExpectedIdentity(record, expected = {}) {
  const mappings = {
    repository: "source_repository",
    buildMode: "build_mode",
    version: "version",
    releaseTag: "release_tag",
    commit: "commit",
    runId: "run_id",
    runAttempt: "run_attempt",
  };
  for (const [inputKey, recordKey] of Object.entries(mappings)) {
    if (expected[inputKey] != null && String(record[recordKey]) !== String(expected[inputKey])) {
      fail(`Notary evidence ${recordKey} does not match the expected workflow identity`);
    }
  }
}

export function validatePendingRecord(record, expected = {}) {
  const common = validateCommonRecord(record, "rho_macos_notary_pending", "pending");
  assertExactKeys(record.submission, ["id", "artifact_name", "size_bytes", "sha256"], "Pending submission");
  record.submission.id = normalizeSubmissionId(record.submission.id);
  if (record.submission.artifact_name !== expectedNotaryArtifactName(record.version, common.artifact_kind)) {
    fail("Pending artifact name is invalid");
  }
  if (!Number.isSafeInteger(record.submission.size_bytes) || record.submission.size_bytes <= 0 || record.submission.size_bytes > MAX_NOTARY_DMG_BYTES) {
    fail("Pending artifact size is invalid");
  }
  if (!SHA256_PATTERN.test(record.submission.sha256)) fail("Pending artifact SHA-256 is invalid");
  validateExpectedIdentity(record, expected);
  return record;
}

function base64url(value) {
  return Buffer.from(value).toString("base64url");
}

export function decodePrivateKeySecret(secret) {
  if (typeof secret !== "string" || secret.length < 16 || secret.length > 32 * 1024 || secret.length % 4 !== 0) {
    fail("Apple API private-key secret is invalid");
  }
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(secret)) fail("Apple API private-key secret is not strict base64");
  const decoded = Buffer.from(secret, "base64").toString("utf8");
  if (!decoded.startsWith("-----BEGIN PRIVATE KEY-----\n") || !decoded.trimEnd().endsWith("-----END PRIVATE KEY-----")) {
    fail("Apple API private-key secret does not contain a PKCS#8 PEM key");
  }
  return decoded;
}

export function createAppleJwt({ issuer, keyId, privateKey, nowSeconds = Math.floor(Date.now() / 1000), lifetimeSeconds = 120 }) {
  const normalizedIssuer = normalizeSubmissionId(issuer, "Apple API issuer");
  if (!API_KEY_ID_PATTERN.test(keyId)) fail("Apple API key ID is invalid");
  if (!Number.isSafeInteger(nowSeconds) || !Number.isSafeInteger(lifetimeSeconds) || lifetimeSeconds < 30 || lifetimeSeconds > 300) {
    fail("Apple API token time bounds are invalid");
  }
  const key = crypto.createPrivateKey(privateKey);
  if (key.asymmetricKeyType !== "ec" || key.asymmetricKeyDetails?.namedCurve !== "prime256v1") {
    fail("Apple API private key must be an EC P-256 key");
  }
  const header = { alg: "ES256", kid: keyId, typ: "JWT" };
  const payload = {
    iss: normalizedIssuer,
    iat: nowSeconds,
    exp: nowSeconds + lifetimeSeconds,
    aud: "appstoreconnect-v1",
  };
  const signingInput = `${base64url(JSON.stringify(header))}.${base64url(JSON.stringify(payload))}`;
  const signature = crypto.sign("sha256", Buffer.from(signingInput), { key, dsaEncoding: "ieee-p1363" });
  if (signature.length !== 64) fail("Apple API token signature is invalid");
  return `${signingInput}.${base64url(signature)}`;
}

function normalizedHeaders(headers = {}) {
  const result = {};
  for (const [name, value] of Object.entries(headers)) {
    if (value == null) continue;
    result[name.toLowerCase()] = Array.isArray(value) ? value.join(", ") : String(value);
  }
  return result;
}

export function httpsRequestBounded({ url, headers = {}, maxBytes, timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS, label = "HTTPS response" }) {
  return new Promise((resolve, reject) => {
    let parsed;
    try {
      parsed = new URL(url);
    } catch {
      reject(new Error(`${label} URL is invalid`));
      return;
    }
    if (parsed.protocol !== "https:" || parsed.username || parsed.password || (parsed.port && parsed.port !== "443")) {
      reject(new Error(`${label} URL must be credential-free HTTPS`));
      return;
    }
    const request = https.request(parsed, {
      method: "GET",
      headers,
      timeout: timeoutMs,
    }, (response) => {
      const responseHeaders = normalizedHeaders(response.headers);
      const contentLength = Number(responseHeaders["content-length"] || 0);
      if (Number.isFinite(contentLength) && contentLength > maxBytes) {
        response.resume();
        reject(new Error(`${label} exceeds its byte budget`));
        return;
      }
      const chunks = [];
      let total = 0;
      response.on("data", (chunk) => {
        total += chunk.length;
        if (total > maxBytes) {
          request.destroy(new Error(`${label} exceeds its byte budget`));
          return;
        }
        chunks.push(chunk);
      });
      response.on("end", () => {
        resolve({ status: response.statusCode || 0, headers: responseHeaders, body: Buffer.concat(chunks) });
      });
    });
    request.on("timeout", () => request.destroy(new Error(`${label} timed out`)));
    request.on("error", (error) => reject(new Error(`${label} request failed: ${error.message}`)));
    request.end();
  });
}

function appleStatusUrl(submissionId) {
  return `${NOTARY_API_ORIGIN}/notary/v2/submissions/${submissionId}`;
}

function appleLogUrl(submissionId) {
  return `${appleStatusUrl(submissionId)}/logs`;
}

function isAllowedDeveloperLogUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    return false;
  }
  const hostname = url.hostname.toLowerCase();
  const allowedHost = hostname === "apple.com"
    || hostname.endsWith(".apple.com")
    || hostname === "itunes.apple.com"
    || hostname.endsWith(".itunes.apple.com")
    || EXACT_DEVELOPER_LOG_HOSTS.has(hostname);
  return url.protocol === "https:" && !url.username && !url.password && (!url.port || url.port === "443") && allowedHost;
}

function parseStatusResponse(body, pending) {
  const response = parseJsonBuffer(body, "Apple notary status response", MAX_NOTARY_STATUS_BYTES);
  const data = assertObject(response.data, "Apple notary status data");
  const attributes = assertObject(data.attributes, "Apple notary status attributes");
  const id = normalizeSubmissionId(data.id, "Apple notary status ID");
  if (id !== pending.submission.id || data.type !== "submissions") fail("Apple notary status identity is invalid");
  const name = boundedString(attributes.name, "Apple notary submission name", 255);
  if (name !== pending.submission.artifact_name) fail("Apple notary submission name does not match the pending artifact");
  const status = boundedString(attributes.status, "Apple notary status", 32);
  if (!APPLE_STATUSES.has(status)) fail(`Apple returned unknown notary status ${status}`);
  const createdDate = boundedString(attributes.createdDate, "Apple notary created date", 64);
  if (!Number.isFinite(Date.parse(createdDate))) fail("Apple notary created date is invalid");
  return { id, name, status, created_date: createdDate };
}

function parseLogUrlResponse(body, pending) {
  const response = parseJsonBuffer(body, "Apple notary log URL response", MAX_NOTARY_LOG_URL_BYTES);
  const data = assertObject(response.data, "Apple notary log URL data");
  const attributes = assertObject(data.attributes, "Apple notary log URL attributes");
  const id = normalizeSubmissionId(data.id, "Apple notary log URL ID");
  if (id !== pending.submission.id || data.type !== "submissionsLog") fail("Apple notary log URL identity is invalid");
  const developerLogUrl = boundedString(attributes.developerLogUrl, "Apple developer log URL", 4096);
  if (!isAllowedDeveloperLogUrl(developerLogUrl)) fail("Apple developer log URL is not an allowed HTTPS URL");
  return developerLogUrl;
}

function validateDeveloperLog(log, pending) {
  const jobId = normalizeSubmissionId(log.jobId, "Apple developer log job ID");
  if (jobId !== pending.submission.id) fail("Apple developer log job ID does not match the pending submission");
  if (log.status !== "Accepted") fail("Apple developer log does not report Accepted");
  if (log.archiveFilename !== pending.submission.artifact_name) fail("Apple developer log archive name does not match the pending artifact");
  if (typeof log.sha256 !== "string" || log.sha256.toLowerCase() !== pending.submission.sha256) {
    fail("Apple developer log SHA-256 does not match the pending artifact");
  }
  const statusSummary = boundedString(log.statusSummary, "Apple developer log status summary", 1024);
  if (log.issues !== null && !Array.isArray(log.issues)) fail("Apple developer log issues must be null or an array");
  if (Array.isArray(log.issues) && log.issues.length > 10_000) fail("Apple developer log issues are unbounded");
  return { status_summary: statusSummary, issue_count: Array.isArray(log.issues) ? log.issues.length : 0 };
}

function retryDelay(response, nowMs, fallbackMs) {
  const header = response.headers?.["retry-after"];
  if (header == null) return fallbackMs;
  if (/^[0-9]{1,5}$/.test(header)) return Math.min(Number(header) * 1000, 300_000);
  const date = Date.parse(header);
  if (Number.isFinite(date)) return Math.min(Math.max(date - nowMs, 1000), 300_000);
  return fallbackMs;
}

function isTransientStatus(status) {
  return status === 429 || (status >= 500 && status <= 599);
}

async function sleepBeforeDeadline({ sleep, now, deadline, delay, reason }) {
  if (now() + delay > deadline) fail(`Timed out waiting for Apple notarization during ${reason}`);
  await sleep(delay);
}

async function requestWithAppleJwt({ url, maxBytes, label, request, issuer, keyId, privateKey, now }) {
  const token = createAppleJwt({ issuer, keyId, privateKey, nowSeconds: Math.floor(now() / 1000) });
  return request({
    url,
    headers: { Authorization: `Bearer ${token}`, Accept: "application/json" },
    maxBytes,
    label,
  });
}

async function retrieveAcceptedLog(options, pending, terminalStatus, deadline) {
  let transientErrors = 0;
  while (true) {
    let urlResponse;
    try {
      urlResponse = await requestWithAppleJwt({
        ...options,
        url: appleLogUrl(pending.submission.id),
        maxBytes: MAX_NOTARY_LOG_URL_BYTES,
        label: "Apple notary log URL response",
      });
    } catch (error) {
      transientErrors += 1;
      if (transientErrors > options.maxTransientErrors) fail(`Apple notary log URL request exhausted retries: ${error.message}`);
      await sleepBeforeDeadline({ ...options, deadline, delay: options.pollIntervalMs, reason: "log URL request" });
      continue;
    }
    if (urlResponse.status !== 200) {
      if (!isTransientStatus(urlResponse.status)) fail(`Apple notary log URL request failed with HTTP ${urlResponse.status}`);
      transientErrors += 1;
      if (transientErrors > options.maxTransientErrors) fail("Apple notary log URL request exhausted transient HTTP retries");
      await sleepBeforeDeadline({
        ...options,
        deadline,
        delay: retryDelay(urlResponse, options.now(), options.pollIntervalMs),
        reason: "log URL response",
      });
      continue;
    }
    const developerLogUrl = parseLogUrlResponse(urlResponse.body, pending);
    let logResponse;
    try {
      logResponse = await options.request({
        url: developerLogUrl,
        headers: { Accept: "application/json" },
        maxBytes: MAX_NOTARY_LOG_BYTES,
        label: "Apple developer log",
      });
    } catch (error) {
      transientErrors += 1;
      if (transientErrors > options.maxTransientErrors) fail(`Apple developer log request exhausted retries: ${error.message}`);
      await sleepBeforeDeadline({ ...options, deadline, delay: options.pollIntervalMs, reason: "developer log request" });
      continue;
    }
    if (logResponse.status !== 200) {
      if (!isTransientStatus(logResponse.status)) fail(`Apple developer log request failed with HTTP ${logResponse.status}`);
      transientErrors += 1;
      if (transientErrors > options.maxTransientErrors) fail("Apple developer log request exhausted transient HTTP retries");
      await sleepBeforeDeadline({
        ...options,
        deadline,
        delay: retryDelay(logResponse, options.now(), options.pollIntervalMs),
        reason: "developer log response",
      });
      continue;
    }
    const log = parseJsonBuffer(logResponse.body, "Apple developer log", MAX_NOTARY_LOG_BYTES);
    const logSummary = validateDeveloperLog(log, pending);
    options.report(`Apple notarization ${pending.submission.id} log is downloaded and bound.`);
    const logName = expectedNotaryLogName(pending.version, recordArtifactKind(pending));
    const accepted = {
      ...commonRecord({
        source_repository: pending.source_repository,
        build_mode: pending.build_mode,
        version: pending.version,
        release_tag: pending.release_tag,
        commit: pending.commit,
        run_id: pending.run_id,
        run_attempt: pending.run_attempt,
      }, "accepted", "rho_macos_notary_accepted", recordArtifactKind(pending)),
      submission: {
        ...pending.submission,
        created_date: terminalStatus.created_date,
        apple_status: terminalStatus.status,
        log: {
          name: logName,
          size_bytes: logResponse.body.length,
          sha256: hashBuffer(logResponse.body),
          status_summary: logSummary.status_summary,
          issue_count: logSummary.issue_count,
        },
      },
    };
    return { accepted, logBytes: logResponse.body };
  }
}

export async function waitForAccepted(pendingInput, {
  issuer,
  keyId,
  privateKey,
  request = httpsRequestBounded,
  sleep = (delay) => new Promise((resolve) => setTimeout(resolve, delay)),
  now = () => Date.now(),
  pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
  maxWaitMs = DEFAULT_MAX_WAIT_MS,
  maxTransientErrors = DEFAULT_MAX_TRANSIENT_ERRORS,
  report = () => {},
} = {}) {
  const pending = validatePendingRecord(structuredClone(pendingInput));
  if (!Number.isSafeInteger(pollIntervalMs) || pollIntervalMs <= 0 || !Number.isSafeInteger(maxWaitMs) || maxWaitMs <= pollIntervalMs) {
    fail("Notary polling bounds are invalid");
  }
  if (!Number.isSafeInteger(maxTransientErrors) || maxTransientErrors < 0 || maxTransientErrors > 32) {
    fail("Notary transient retry bound is invalid");
  }
  if (typeof report !== "function") fail("Notary progress reporter is invalid");
  boundedString(issuer, "Apple API issuer", 36);
  boundedString(keyId, "Apple API key ID", 10);
  if (typeof privateKey !== "string" || privateKey.length < 64 || privateKey.length > 16 * 1024 || privateKey.includes("\u0000")) {
    fail("Apple API private key is invalid");
  }
  const options = { issuer, keyId, privateKey, request, sleep, now, pollIntervalMs, maxTransientErrors, report };
  const deadline = now() + maxWaitMs;
  let transientErrors = 0;
  while (true) {
    let response;
    try {
      response = await requestWithAppleJwt({
        ...options,
        url: appleStatusUrl(pending.submission.id),
        maxBytes: MAX_NOTARY_STATUS_BYTES,
        label: "Apple notary status response",
      });
    } catch (error) {
      transientErrors += 1;
      if (transientErrors > maxTransientErrors) fail(`Apple notary status request exhausted retries: ${error.message}`);
      report(`Apple notarization ${pending.submission.id} status request is temporarily unavailable; retry ${transientErrors}/${maxTransientErrors}.`);
      await sleepBeforeDeadline({ ...options, deadline, delay: pollIntervalMs, reason: "status request" });
      continue;
    }
    if (response.status === 200) {
      const status = parseStatusResponse(response.body, pending);
      transientErrors = 0;
      if (status.status === "Accepted") {
        report(`Apple notarization ${pending.submission.id} is Accepted; retrieving its log.`);
        return retrieveAcceptedLog(options, pending, status, deadline);
      }
      if (status.status === "Invalid" || status.status === "Rejected") {
        fail(`Apple notarization ended with ${status.status}`);
      }
      report(`Apple notarization ${pending.submission.id} is In Progress.`);
      await sleepBeforeDeadline({ ...options, deadline, delay: pollIntervalMs, reason: "In Progress status" });
      continue;
    }
    if (!isTransientStatus(response.status)) fail(`Apple notary status request failed with HTTP ${response.status}`);
    transientErrors += 1;
    if (transientErrors > maxTransientErrors) fail("Apple notary status request exhausted transient HTTP retries");
    report(`Apple notarization ${pending.submission.id} returned transient HTTP ${response.status}; retry ${transientErrors}/${maxTransientErrors}.`);
    await sleepBeforeDeadline({
      ...options,
      deadline,
      delay: retryDelay(response, now(), pollIntervalMs),
      reason: "transient status response",
    });
  }
}

export function validateAcceptedRecord(record, pendingInput, expected = {}) {
  const pending = validatePendingRecord(structuredClone(pendingInput), expected);
  const common = validateCommonRecord(record, "rho_macos_notary_accepted", "accepted");
  validateExpectedIdentity(record, expected);
  for (const key of ["source_repository", "build_mode", "version", "release_tag", "commit", "run_id", "run_attempt", "platform"]) {
    if (record[key] !== pending[key]) fail(`Accepted notary evidence ${key} does not match the pending record`);
  }
  if (common.artifact_kind !== recordArtifactKind(pending)) {
    fail("Accepted notary evidence artifact kind does not match the pending record");
  }
  assertExactKeys(record.submission, [
    "id",
    "artifact_name",
    "size_bytes",
    "sha256",
    "created_date",
    "apple_status",
    "log",
  ], "Accepted submission");
  for (const key of ["id", "artifact_name", "size_bytes", "sha256"]) {
    if (record.submission[key] !== pending.submission[key]) fail(`Accepted submission ${key} does not match pending evidence`);
  }
  if (record.submission.apple_status !== "Accepted" || !Number.isFinite(Date.parse(record.submission.created_date))) {
    fail("Accepted submission status or date is invalid");
  }
  assertExactKeys(record.submission.log, ["name", "size_bytes", "sha256", "status_summary", "issue_count"], "Accepted log");
  if (record.submission.log.name !== expectedNotaryLogName(record.version, common.artifact_kind)) {
    fail("Accepted log name is invalid");
  }
  if (!Number.isSafeInteger(record.submission.log.size_bytes) || record.submission.log.size_bytes <= 0 || record.submission.log.size_bytes > MAX_NOTARY_LOG_BYTES) {
    fail("Accepted log size is invalid");
  }
  if (!SHA256_PATTERN.test(record.submission.log.sha256)) fail("Accepted log SHA-256 is invalid");
  boundedString(record.submission.log.status_summary, "Accepted log status summary", 1024);
  if (!Number.isSafeInteger(record.submission.log.issue_count) || record.submission.log.issue_count < 0 || record.submission.log.issue_count > 10_000) {
    fail("Accepted log issue count is invalid");
  }
  return record;
}

export function verifyFinalizerInputs({ pendingPath, acceptedPath, logPath, dmgPath, artifactPath, expected = {} }) {
  const pending = validatePendingRecord(readJsonFile(pendingPath, MAX_NOTARY_EVIDENCE_BYTES, "Pending notary evidence"), expected);
  const accepted = validateAcceptedRecord(
    readJsonFile(acceptedPath, MAX_NOTARY_EVIDENCE_BYTES, "Accepted notary evidence"),
    pending,
    expected,
  );
  const logBytes = readRegularFile(logPath, MAX_NOTARY_LOG_BYTES, "Apple developer log");
  if (path.basename(logPath) !== accepted.submission.log.name
    || logBytes.length !== accepted.submission.log.size_bytes
    || hashBuffer(logBytes) !== accepted.submission.log.sha256) {
    fail("Apple developer log does not match accepted evidence");
  }
  validateDeveloperLog(parseJsonBuffer(logBytes, "Apple developer log", MAX_NOTARY_LOG_BYTES), pending);
  const submittedArtifactPath = artifactPath ?? dmgPath;
  if (!submittedArtifactPath) fail("Notary finalizer artifact is missing");
  const artifact = hashFile(submittedArtifactPath);
  if (path.basename(submittedArtifactPath) !== pending.submission.artifact_name
    || artifact.size_bytes !== pending.submission.size_bytes
    || artifact.sha256 !== pending.submission.sha256) {
    fail("Submitted notarization artifact does not match pending notary evidence");
  }
  return { pending, accepted };
}

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value == null) fail(`Invalid argument at ${key || "end of input"}`);
    const name = key.slice(2).replaceAll("-", "_");
    if (result[name] != null) fail(`Duplicate argument ${key}`);
    result[name] = value;
  }
  return result;
}

function required(args, name) {
  if (!args[name]) fail(`Missing --${name.replaceAll("_", "-")}`);
  return args[name];
}

function expectedFromArgs(args, includeRunAttempt = true) {
  const expected = {
    repository: required(args, "repository"),
    buildMode: required(args, "build_mode"),
    version: required(args, "version"),
    releaseTag: required(args, "tag"),
    commit: required(args, "commit"),
    runId: required(args, "run_id"),
  };
  if (includeRunAttempt) expected.runAttempt = required(args, "run_attempt");
  return expected;
}

function artifactFromArgs(args) {
  if (args.dmg && args.artifact) fail("Use either --dmg or --artifact, not both");
  if (args.dmg) {
    if (args.artifact_kind != null && args.artifact_kind !== "dmg") {
      fail("--dmg requires the dmg artifact kind");
    }
    return { artifactPath: args.dmg, artifactKind: "dmg" };
  }
  return {
    artifactPath: required(args, "artifact"),
    artifactKind: required(args, "artifact_kind"),
  };
}

async function main(argv) {
  const [mode, ...rest] = argv;
  const args = parseArgs(rest);
  if (mode === "submission") {
    const artifact = artifactFromArgs(args);
    const pending = createPendingRecord({
      receiptPath: required(args, "receipt"),
      ...artifact,
      ...expectedFromArgs(args),
    });
    writeJsonExclusive(required(args, "output"), pending, "Pending notary evidence");
    process.stdout.write(`Recorded pending Apple notarization ${pending.submission.id}.\n`);
    return;
  }
  if (mode === "wait") {
    const expected = expectedFromArgs(args, false);
    const pending = validatePendingRecord(
      readJsonFile(required(args, "pending"), MAX_NOTARY_EVIDENCE_BYTES, "Pending notary evidence"),
      expected,
    );
    const privateKey = decodePrivateKeySecret(required(process.env, "APPLE_API_PRIVATE_KEY"));
    const pollIntervalMs = args.poll_interval_ms == null ? DEFAULT_POLL_INTERVAL_MS : Number(args.poll_interval_ms);
    const maxWaitMs = args.max_wait_ms == null ? DEFAULT_MAX_WAIT_MS : Number(args.max_wait_ms);
    if (pollIntervalMs < 30_000 || pollIntervalMs > 300_000 || maxWaitMs < 60_000 || maxWaitMs > 20_700_000) {
      fail("CLI notary polling bounds are invalid");
    }
    const result = await waitForAccepted(pending, {
      issuer: required(process.env, "APPLE_API_ISSUER"),
      keyId: required(process.env, "APPLE_API_KEY"),
      privateKey,
      pollIntervalMs,
      maxWaitMs,
      report: (message) => process.stdout.write(`${message}\n`),
    });
    const logOutput = required(args, "log_output");
    if (path.basename(logOutput) !== result.accepted.submission.log.name) fail("Developer log output name is invalid");
    writeExclusive(logOutput, result.logBytes, MAX_NOTARY_LOG_BYTES, "Apple developer log");
    writeJsonExclusive(required(args, "output"), result.accepted, "Accepted notary evidence");
    process.stdout.write(`Apple notarization ${pending.submission.id} is Accepted and log-bound.\n`);
    return;
  }
  if (mode === "verify") {
    const artifact = artifactFromArgs(args);
    const verified = verifyFinalizerInputs({
      pendingPath: required(args, "pending"),
      acceptedPath: required(args, "accepted"),
      logPath: required(args, "log"),
      ...artifact,
      expected: expectedFromArgs(args, false),
    });
    process.stdout.write(`Verified immutable notarization inputs for ${verified.pending.submission.id}.\n`);
    return;
  }
  fail(`Unsupported macOS notary mode ${mode || "<empty>"}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
