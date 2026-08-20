# Coding Practice Arena — Phase 1: Judge Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a self-hosted Judge0 instance and a fully tested Rust client that compiles and runs untrusted code in 14 languages under hard resource limits, with no user-facing surface.

**Architecture:** Judge0 runs as an isolated `docker compose` stack with its own Postgres and Redis, bound to loopback, holding no application secrets. A `services/practice/judge.rs` client posts one Judge0 *batch* per submission — one entry per test case — polls the batch tokens to completion, and folds the per-case results into a single verdict. Verdict aggregation and the language table are pure functions with no I/O, so they are unit-tested directly; the HTTP client is tested against `wiremock`; the sandbox and the 14 starter templates are verified by `#[ignore]`-gated tests that run against a live judge.

**Tech Stack:** Rust 2024, Salvo 0.95, SeaORM 2.0, `reqwest` 0.12, `base64` 0.22, `thiserror` 2, `wiremock` 0.6 (dev), Judge0 CE 1.13.x, Docker Compose.

**Spec:** `docs/superpowers/specs/2026-08-20-coding-practice-judge-design.md`

## Global Constraints

- **No AI attribution in any commit.** No `Co-Authored-By`, no mention of Claude, Anthropic or AI. Short conventional-commit subjects: `feat:`, `fix:`, `docs:`, `test:`, `chore:`.
- **Never `git push`.** The user pushes.
- **Commit granularly** — one commit per task, at minimum. Every task below ends in a commit.
- **Stage explicit paths only.** Never `git add -A` or `git add .`. This repo has a large unrelated uncommitted working tree (an in-progress `users` → `user_profiles` rename); sweeping it into a commit is a defect.
- `cargo fmt --all` and `cargo clippy` must pass before every commit. `.rustfmt.toml` sets `max_width = 100`.
- `enable_network` is `false` on every execution, without exception (spec §13).
- Stored `stdout`, `stderr` and `compile_output` are truncated to 8 KB each (spec §13).
- Judge0 language ids are **never trusted from documentation** — they are verified against `GET /languages` at boot (spec §7).
- Judge0 is bound to `127.0.0.1` and never publicly exposed (spec §13).

---

## File Structure

| File | Responsibility |
|---|---|
| `deploy/judge0/docker-compose.yml` | Judge0 stack: api, workers, own postgres, own redis. Loopback-bound, privileged workers, resource-capped. |
| `deploy/judge0/judge0.conf` | Judge0 configuration: hard resource ceilings, auth token, network pinned off. |
| `deploy/judge0/README.md` | Host prerequisites (cgroup v1), bring-up, smoke test, troubleshooting. |
| `src/config/mod.rs` | *Modified.* Adds the `JUDGE0_*` fields to `Config`. |
| `src/services/practice/mod.rs` | Module root for the practice area. |
| `src/services/practice/languages.rs` | The 14-language table, lookup by slug, entry-point guards. Pure, no I/O. |
| `src/services/practice/judge.rs` | Judge0 HTTP client, request building, batch polling. |
| `src/services/practice/verdict.rs` | Per-case results → one verdict. Pure, no I/O. |
| `tests/config_judge.rs` | `Config` defaults and env parsing for the judge settings. |
| `tests/languages.rs` | Language-table invariants and the entry-point guards. |
| `tests/verdict.rs` | Exhaustive verdict-aggregation and truncation tests. Pure, no server. |
| `tests/judge_client.rs` | `wiremock`-backed tests for request shape, polling and language validation. |
| `tests/judge_live.rs` | `#[ignore]`-gated tests against a real Judge0: 14 starter templates, adversarial sandbox suite. |

`verdict.rs` is split from `judge.rs` deliberately: aggregation is the most rule-dense logic in the feature and has zero I/O, so keeping it separate means it is exhaustively unit-testable without a mock server, and `judge.rs` stays a thin transport layer.

---

## Task 1: Judge0 stack and host verification

**Files:**
- Create: `deploy/judge0/docker-compose.yml`
- Create: `deploy/judge0/judge0.conf`
- Create: `deploy/judge0/README.md`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: a Judge0 API reachable at `http://127.0.0.1:2358` requiring header `X-Auth-Token: <JUDGE0_AUTH_TOKEN>`. Every later task depends on this being up.

This task has no Rust and no `cargo test`. Its deliverable is a running judge, verified by `curl`.

- [ ] **Step 1: Check the host's cgroup version**

Run:

```bash
stat -fc %T /sys/fs/cgroup/
```

Expected: `tmpfs` means cgroup v1 (Judge0 1.13.x works as-is). `cgroup2fs` means cgroup v2, and Judge0 1.13.0 will fail to start.

If the result is `cgroup2fs`, either use image `judge0/judge0:1.13.1` (which supports v2) or boot the host with cgroup v1 by adding `systemd.unified_cgroup_hierarchy=0` to the kernel command line and rebooting. Do not continue until this is resolved — every later task needs a working judge.

- [ ] **Step 2: Write the compose file**

Create `deploy/judge0/docker-compose.yml`:

```yaml
# Judge0 runs as its own stack with its own Postgres and Redis. It holds no
# application secrets: a full compromise of this stack yields a machine that
# executes untrusted code, which is already its purpose.
services:
  server:
    image: judge0/judge0:1.13.1
    volumes:
      - ./judge0.conf:/judge0.conf:ro
    # Loopback only. Judge0 ships no authentication by default; anyone who can
    # reach this port can execute arbitrary code on the host.
    ports:
      - "127.0.0.1:2358:2358"
    privileged: true
    restart: always
    depends_on:
      - db
      - redis

  workers:
    image: judge0/judge0:1.13.1
    command: ["./scripts/workers"]
    volumes:
      - ./judge0.conf:/judge0.conf:ro
    privileged: true
    restart: always
    depends_on:
      - db
      - redis
    # A fork bomb inside the sandbox starves the judge, never the API host.
    deploy:
      resources:
        limits:
          cpus: "2.0"
          memory: 2g

  db:
    image: postgres:16-alpine
    env_file: judge0.conf
    volumes:
      - judge0-db:/var/lib/postgresql/data
    restart: always

  redis:
    image: redis:7-alpine
    command:
      [
        "bash",
        "-c",
        'docker-entrypoint.sh --appendonly no --requirepass "$$REDIS_PASSWORD"',
      ]
    env_file: judge0.conf
    restart: always

volumes:
  judge0-db:
```

- [ ] **Step 3: Write the Judge0 configuration**

Create `deploy/judge0/judge0.conf`. Replace both secret values with freshly generated ones — `openssl rand -hex 32` for each.

```conf
################################################################################
# Judge0 configuration for the SlintTech practice arena.
#
# The MAX_* ceilings below are hard limits enforced by Judge0 itself. The
# application also sends per-execution limits, but limits enforced only by the
# caller are not limits: a bug in request construction must not be able to ask
# for 60 seconds and receive it.
################################################################################

# Authentication. Second layer of defence behind loopback binding.
AUTHN_HEADER=X-Auth-Token
AUTHN_TOKEN=replace-me-with-openssl-rand-hex-32

# Network access is pinned off globally, so no submission can enable it.
ALLOW_ENABLE_NETWORK=false

# Hard resource ceilings.
MAX_CPU_TIME_LIMIT=10
MAX_CPU_EXTRA_TIME=2
MAX_WALL_TIME_LIMIT=20
MAX_MEMORY_LIMIT=512000
MAX_MAX_PROCESSES_AND_OR_THREADS=120
MAX_MAX_FILE_SIZE=4096
MAX_STACK_LIMIT=128000
MAX_NUMBER_OF_RUNS=1

# One batch carries every test case of one submission.
MAX_SUBMISSION_BATCH_SIZE=100
MAX_QUEUE_SIZE=200

# Judge0's own datastores. Not the application database.
POSTGRES_HOST=db
POSTGRES_DB=judge0
POSTGRES_USER=judge0
POSTGRES_PASSWORD=replace-me-with-openssl-rand-hex-32

REDIS_HOST=redis
REDIS_PASSWORD=replace-me-with-openssl-rand-hex-32
```

- [ ] **Step 4: Keep the configured secrets out of git**

Append to `.gitignore`:

```gitignore

# Judge0 runtime configuration contains generated secrets.
deploy/judge0/judge0.conf.local
```

Commit `judge0.conf` with the `replace-me-…` placeholders in place; keep the filled-in copy as `judge0.conf.local` and bind that one in compose on the deployed host.

- [ ] **Step 5: Bring the stack up**

Run:

```bash
cd deploy/judge0 && docker compose up -d && sleep 15 && docker compose ps
```

Expected: four services, all `running`. If `workers` is restarting, read `docker compose logs workers` — a cgroup error here means Step 1 was not resolved.

- [ ] **Step 6: Smoke-test the API**

Run:

```bash
TOKEN=$(grep '^AUTHN_TOKEN=' judge0.conf | cut -d= -f2)
curl -s -H "X-Auth-Token: $TOKEN" http://127.0.0.1:2358/about
```

Expected: a JSON body containing `"version"`.

- [ ] **Step 7: Confirm it refuses unauthenticated calls**

Run:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:2358/about
```

Expected: `401`. If this prints `200`, `AUTHN_TOKEN` was not picked up — fix it before continuing, because the loopback binding is then the only thing protecting arbitrary code execution.

- [ ] **Step 8: Capture the real language ids**

Run:

```bash
curl -s -H "X-Auth-Token: $TOKEN" http://127.0.0.1:2358/languages | python3 -m json.tool
```

Save this output. Task 3 hardcodes these ids and Task 8 verifies them at boot; the numbers in Task 3 are correct for Judge0 CE 1.13.x but **must be checked against this output**, because they shift between releases.

- [ ] **Step 9: Write the deployment README**

Create `deploy/judge0/README.md`:

```markdown
# Judge0 stack

Executes untrusted user code for the practice arena. See
`docs/superpowers/specs/2026-08-20-coding-practice-judge-design.md` §13 for the
security rationale behind this topology.

## Host prerequisites

- Docker with Compose v2.
- Ability to run `privileged` containers — `isolate` uses cgroups and
  namespaces directly. Shared PaaS hosts cannot run this.
- cgroup v1, or Judge0 1.13.1+ for cgroup v2. Check with
  `stat -fc %T /sys/fs/cgroup/`: `tmpfs` is v1, `cgroup2fs` is v2. To force v1,
  add `systemd.unified_cgroup_hierarchy=0` to the kernel command line and
  reboot. This is the most common Judge0 startup failure.

## Bring-up

1. `cp judge0.conf judge0.conf.local`
2. Replace all three `replace-me-…` values with `openssl rand -hex 32` output.
3. Point the compose `volumes` entries at `judge0.conf.local`.
4. `docker compose up -d`

## Verify

```bash
TOKEN=$(grep '^AUTHN_TOKEN=' judge0.conf.local | cut -d= -f2)
curl -s -H "X-Auth-Token: $TOKEN" http://127.0.0.1:2358/about     # expect version JSON
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:2358/about  # expect 401
```

## Security invariants

- The API is bound to `127.0.0.1`. It must never be published on a public
  interface. Judge0 has no authentication beyond `AUTHN_TOKEN`.
- This stack holds no `JWT_SECRET`, no `PAYSTACK_SECRET_KEY`, no
  `CLOUDINARY_API_SECRET`, and no application `DATABASE_URL`.
- `ALLOW_ENABLE_NETWORK=false` is global, so no submission can enable
  networking regardless of what the application sends.
```

- [ ] **Step 10: Commit**

```bash
git add deploy/judge0/docker-compose.yml deploy/judge0/judge0.conf deploy/judge0/README.md .gitignore
git commit -m "feat: add self-hosted Judge0 stack for code execution"
```

---

## Task 2: Dependencies and judge configuration

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config/mod.rs`
- Modify: `.env.example`
- Create: `tests/config_judge.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Config` gains `judge0_url: String`, `judge0_auth_token: Option<String>`, `judge_max_inflight: usize`, `judge_worker_tick_ms: u64`, `judge_max_attempts: i32`, `judge_stale_claim_seconds: i64`, `judge_batch_deadline_seconds: u64`, `judge_batch_poll_start_ms: u64`, `judge_batch_poll_max_ms: u64`, `practice_max_source_bytes: usize`. Tasks 6, 7 and 8 construct their client from these.

- [ ] **Step 1: Write the failing test**

Create `tests/config_judge.rs`:

```rust
//! `Config` must supply working defaults for every judge setting, so a deploy
//! that sets only `JUDGE0_URL` still runs with the limits the spec requires.

use slinttech_server::config::Config;

/// `Config::from_env` reads process-global state, so these tests must not run
/// concurrently with each other.
fn with_env<T>(vars: &[(&str, Option<&str>)], test: impl FnOnce() -> T) -> T {
    use std::sync::Mutex;
    static GUARD: Mutex<()> = Mutex::new(());
    let _lock = GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let previous: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
        .collect();

    for (key, value) in vars {
        match value {
            Some(value) => unsafe { std::env::set_var(key, value) },
            None => unsafe { std::env::remove_var(key) },
        }
    }

    let result = test();

    for (key, value) in previous {
        match value {
            Some(value) => unsafe { std::env::set_var(&key, value) },
            None => unsafe { std::env::remove_var(&key) },
        }
    }

    result
}

#[test]
fn judge_settings_fall_back_to_spec_defaults() {
    let config = with_env(
        &[
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("JWT_SECRET", Some("test-secret")),
            ("JUDGE0_URL", None),
            ("JUDGE_MAX_INFLIGHT", None),
            ("JUDGE_MAX_ATTEMPTS", None),
            ("PRACTICE_MAX_SOURCE_BYTES", None),
        ],
        Config::from_env,
    );

    assert_eq!(config.judge0_url, "http://127.0.0.1:2358");
    assert_eq!(config.judge_max_inflight, 4);
    assert_eq!(config.judge_worker_tick_ms, 250);
    assert_eq!(config.judge_max_attempts, 3);
    assert_eq!(config.judge_stale_claim_seconds, 300);
    assert_eq!(config.judge_batch_deadline_seconds, 60);
    assert_eq!(config.judge_batch_poll_start_ms, 100);
    assert_eq!(config.judge_batch_poll_max_ms, 1000);
    assert_eq!(config.practice_max_source_bytes, 65536);
    assert!(config.judge0_auth_token.is_none());
}

#[test]
fn judge_settings_read_from_the_environment() {
    let config = with_env(
        &[
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("JWT_SECRET", Some("test-secret")),
            ("JUDGE0_URL", Some("http://judge.internal:2358")),
            ("JUDGE0_AUTH_TOKEN", Some("secret-token")),
            ("JUDGE_MAX_INFLIGHT", Some("8")),
            ("PRACTICE_MAX_SOURCE_BYTES", Some("1024")),
        ],
        Config::from_env,
    );

    assert_eq!(config.judge0_url, "http://judge.internal:2358");
    assert_eq!(config.judge0_auth_token.as_deref(), Some("secret-token"));
    assert_eq!(config.judge_max_inflight, 8);
    assert_eq!(config.practice_max_source_bytes, 1024);
}

#[test]
fn trailing_slash_is_stripped_from_the_judge_url() {
    let config = with_env(
        &[
            ("DATABASE_URL", Some("postgres://localhost/test")),
            ("JWT_SECRET", Some("test-secret")),
            ("JUDGE0_URL", Some("http://judge.internal:2358/")),
        ],
        Config::from_env,
    );

    // Paths are joined as `{base}/submissions/batch`; a trailing slash here
    // would produce a double slash and a 404 that is tedious to diagnose.
    assert_eq!(config.judge0_url, "http://judge.internal:2358");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test config_judge`
Expected: FAIL — compilation error, `no field 'judge0_url' on type 'Config'`.

- [ ] **Step 3: Add the dependencies**

In `Cargo.toml`, add to `[dependencies]`:

```toml
# Judge0 payloads are base64-encoded so that source and test data survive
# transport regardless of encoding.
base64 = "0.22"

# The judge client's error type has several variants that wrap a source error;
# hand-written Display and Error impls for those would be pure boilerplate.
thiserror = "2"
```

And add a new section at the end of the file:

```toml
[dev-dependencies]
# Mock HTTP server for judge client tests, so the unit suite needs no live judge.
wiremock = "0.6"
```

- [ ] **Step 4: Add the fields to Config**

In `src/config/mod.rs`, add to the `Config` struct after `cloudinary_api_secret`:

```rust
    // Judge0 execution engine. See the design spec §15.
    pub judge0_url: String,
    pub judge0_auth_token: Option<String>,
    pub judge_max_inflight: usize,
    pub judge_worker_tick_ms: u64,
    pub judge_max_attempts: i32,
    pub judge_stale_claim_seconds: i64,
    pub judge_batch_deadline_seconds: u64,
    pub judge_batch_poll_start_ms: u64,
    pub judge_batch_poll_max_ms: u64,
    pub practice_max_source_bytes: usize,
```

Add this helper below the `impl Config` block's opening brace, above `from_env`:

```rust
    /// Reads a numeric setting, falling back to `default` when unset.
    ///
    /// Unlike the required settings above this does not panic on a malformed
    /// value: an unparseable tuning knob should not stop the server booting.
    fn env_parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
        match env::var(key) {
            Ok(raw) => raw.trim().parse().unwrap_or(default),
            Err(_) => default,
        }
    }
```

And add to the struct literal in `from_env`, after `cloudinary_api_secret`:

```rust
            judge0_url: env::var("JUDGE0_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:2358".to_string())
                .trim_end_matches('/')
                .to_string(),
            judge0_auth_token: env::var("JUDGE0_AUTH_TOKEN")
                .ok()
                .filter(|token| !token.trim().is_empty()),
            judge_max_inflight: Self::env_parsed("JUDGE_MAX_INFLIGHT", 4),
            judge_worker_tick_ms: Self::env_parsed("JUDGE_WORKER_TICK_MS", 250),
            judge_max_attempts: Self::env_parsed("JUDGE_MAX_ATTEMPTS", 3),
            judge_stale_claim_seconds: Self::env_parsed("JUDGE_STALE_CLAIM_SECONDS", 300),
            judge_batch_deadline_seconds: Self::env_parsed("JUDGE_BATCH_DEADLINE_SECONDS", 60),
            judge_batch_poll_start_ms: Self::env_parsed("JUDGE_BATCH_POLL_START_MS", 100),
            judge_batch_poll_max_ms: Self::env_parsed("JUDGE_BATCH_POLL_MAX_MS", 1000),
            practice_max_source_bytes: Self::env_parsed("PRACTICE_MAX_SOURCE_BYTES", 65536),
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test config_judge`
Expected: PASS, 3 tests.

- [ ] **Step 6: Document the settings**

Append to `.env.example`:

```dotenv

# Judge0 code execution engine
JUDGE0_URL=http://127.0.0.1:2358
JUDGE0_AUTH_TOKEN=
JUDGE_MAX_INFLIGHT=4
JUDGE_WORKER_TICK_MS=250
JUDGE_MAX_ATTEMPTS=3
JUDGE_STALE_CLAIM_SECONDS=300
JUDGE_BATCH_DEADLINE_SECONDS=60
JUDGE_BATCH_POLL_START_MS=100
JUDGE_BATCH_POLL_MAX_MS=1000

# Practice arena limits
PRACTICE_MAX_SOURCE_BYTES=65536
PRACTICE_SUBMIT_RATE_PER_MINUTE=12
PRACTICE_SUBMIT_RATE_PER_HOUR=60
PRACTICE_RUN_RATE_PER_MINUTE=30
PRACTICE_MAX_CONCURRENT_RUNS=4
LEADERBOARD_BROADCAST_INTERVAL_MS=1000
LEADERBOARD_TOP_N=50
```

- [ ] **Step 7: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add Cargo.toml Cargo.lock src/config/mod.rs .env.example tests/config_judge.rs
git commit -m "feat: add judge0 configuration to Config"
```

---

## Task 3: The language table

**Files:**
- Create: `src/services/practice/mod.rs`
- Create: `src/services/practice/languages.rs`
- Modify: `src/services/mod.rs`
- Create: `tests/languages.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Language { pub slug: &'static str, pub display_name: &'static str, pub judge0_id: i32, pub judge0_name_contains: &'static str, pub monaco_mode: &'static str, pub file_extension: &'static str, pub compiler_options: Option<&'static str>, pub starter_code: &'static str }`
  - `pub const LANGUAGES: &[Language]` — exactly 14 entries.
  - `pub fn by_slug(slug: &str) -> Option<&'static Language>`
  - Task 5 uses `Language::judge0_id` and `compiler_options`; Task 8 uses `judge0_id` and `judge0_name_contains`; Task 9 uses `starter_code`.

- [ ] **Step 1: Write the failing test**

Create `tests/languages.rs`:

```rust
//! The language table is the single source of truth for what the arena can run.
//! These tests guard the invariants that make it safe to index by slug and to
//! validate against Judge0 at boot.

use slinttech_server::services::practice::languages::{self, LANGUAGES};

#[test]
fn all_fourteen_languages_are_present() {
    assert_eq!(LANGUAGES.len(), 14);
}

#[test]
fn slugs_are_unique() {
    let mut slugs: Vec<&str> = LANGUAGES.iter().map(|language| language.slug).collect();
    slugs.sort_unstable();
    let count = slugs.len();
    slugs.dedup();
    assert_eq!(slugs.len(), count, "duplicate language slug");
}

#[test]
fn judge0_ids_are_unique() {
    let mut ids: Vec<i32> = LANGUAGES.iter().map(|language| language.judge0_id).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "duplicate judge0 language id");
}

#[test]
fn lookup_by_slug_finds_a_known_language() {
    let language = languages::by_slug("rust").expect("rust must be configured");
    assert_eq!(language.display_name, "Rust");
    assert_eq!(language.monaco_mode, "rust");
    assert_eq!(language.file_extension, "rs");
}

#[test]
fn lookup_by_slug_rejects_an_unknown_language() {
    assert!(languages::by_slug("brainfuck").is_none());
    assert!(languages::by_slug("").is_none());
    assert!(languages::by_slug("RUST").is_none(), "lookup is case-sensitive");
}

#[test]
fn cpp_requests_the_cpp17_standard() {
    // GCC 9.2 defaults to gnu++14, so "C++17" is only true if we pass the flag.
    let cpp = languages::by_slug("cpp").expect("cpp must be configured");
    assert_eq!(cpp.compiler_options, Some("-std=c++17"));
}

#[test]
fn every_language_has_a_non_empty_starter_template() {
    for language in LANGUAGES {
        assert!(
            !language.starter_code.trim().is_empty(),
            "{} has no starter template",
            language.slug
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test languages`
Expected: FAIL — `unresolved import slinttech_server::services::practice`.

- [ ] **Step 3: Create the module root**

Create `src/services/practice/mod.rs`:

```rust
//! The coding practice arena: problem catalog, sandboxed judging and ranking.
//!
//! See `docs/superpowers/specs/2026-08-20-coding-practice-judge-design.md`.

pub mod languages;
```

In `src/services/mod.rs`, add:

```rust
pub mod practice;
```

- [ ] **Step 4: Write the language table**

Create `src/services/practice/languages.rs`:

```rust
//! The languages the arena can compile and run.
//!
//! This is configuration in code rather than a database table on purpose: a new
//! language is never a pure data change, because it also needs a Monaco mode and
//! a starter template shipped with the frontend.
//!
//! `judge0_id` values below are correct for Judge0 CE 1.13.x. They shift between
//! Judge0 releases, so they are verified against `GET /languages` at boot rather
//! than trusted — see `judge::validate_languages`.

/// A language the arena can compile and run, as configured for our Judge0 instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// Stable identifier used on the wire and stored on every submission.
    pub slug: &'static str,
    /// Name shown in the language picker.
    pub display_name: &'static str,
    /// Judge0 language id. Verified at boot.
    pub judge0_id: i32,
    /// Substring the Judge0 language name must contain for the id to be
    /// considered correct. Guards against an upgrade silently repointing an id
    /// at a different compiler.
    pub judge0_name_contains: &'static str,
    /// Monaco editor language mode id.
    pub monaco_mode: &'static str,
    /// Source file extension, used for editor hints and download filenames.
    pub file_extension: &'static str,
    /// Extra compiler flags, passed through to Judge0 verbatim.
    pub compiler_options: Option<&'static str>,
    /// Default stdin/stdout skeleton, offered when a problem has no override.
    pub starter_code: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        slug: "python",
        display_name: "Python 3",
        judge0_id: 71,
        judge0_name_contains: "Python",
        monaco_mode: "python",
        file_extension: "py",
        compiler_options: None,
        starter_code: r#"import sys


def main() -> None:
    data = sys.stdin.read().split()
    # your code here


main()
"#,
    },
    Language {
        slug: "javascript",
        display_name: "JavaScript (Node.js)",
        judge0_id: 63,
        judge0_name_contains: "JavaScript",
        monaco_mode: "javascript",
        file_extension: "js",
        compiler_options: None,
        starter_code: r#"const data = require("fs")
  .readFileSync(0, "utf8")
  .split(/\s+/)
  .filter((token) => token.length > 0);

function main() {
  // your code here
}

main();
"#,
    },
    Language {
        slug: "typescript",
        display_name: "TypeScript (Node.js)",
        judge0_id: 74,
        judge0_name_contains: "TypeScript",
        monaco_mode: "typescript",
        file_extension: "ts",
        compiler_options: None,
        // Judge0 runs `tsc` without @types/node, so `require` is declared here
        // rather than imported. Without this the template fails to typecheck.
        starter_code: r#"declare function require(name: string): any;

const data: string[] = require("fs")
  .readFileSync(0, "utf8")
  .split(/\s+/)
  .filter((token: string) => token.length > 0);

function main(): void {
  // your code here
}

main();
"#,
    },
    Language {
        slug: "ruby",
        display_name: "Ruby",
        judge0_id: 72,
        judge0_name_contains: "Ruby",
        monaco_mode: "ruby",
        file_extension: "rb",
        compiler_options: None,
        starter_code: r#"data = $stdin.read.split

def main(data)
  # your code here
end

main(data)
"#,
    },
    Language {
        slug: "php",
        display_name: "PHP",
        judge0_id: 68,
        judge0_name_contains: "PHP",
        monaco_mode: "php",
        file_extension: "php",
        compiler_options: None,
        starter_code: r#"<?php

$data = preg_split('/\s+/', trim(stream_get_contents(STDIN)), -1, PREG_SPLIT_NO_EMPTY);

function main(array $data): void
{
    // your code here
}

main($data);
"#,
    },
    Language {
        slug: "java",
        display_name: "Java",
        judge0_id: 62,
        judge0_name_contains: "Java",
        monaco_mode: "java",
        file_extension: "java",
        compiler_options: None,
        // Judge0 writes the source to Main.java, so the public class must be
        // Main. `validate_entry_point` rejects anything else before it is sent.
        starter_code: r#"import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

public class Main {
    public static void main(String[] args) throws IOException {
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        List<String> data = new ArrayList<>();
        String line;
        while ((line = reader.readLine()) != null) {
            for (String token : line.trim().split("\\s+")) {
                if (!token.isEmpty()) {
                    data.add(token);
                }
            }
        }

        // your code here
    }
}
"#,
    },
    Language {
        slug: "kotlin",
        display_name: "Kotlin",
        judge0_id: 78,
        judge0_name_contains: "Kotlin",
        monaco_mode: "kotlin",
        file_extension: "kt",
        compiler_options: None,
        starter_code: r#"fun main() {
    val data = generateSequence(::readLine)
        .flatMap { line -> line.trim().split(Regex("\\s+")).asSequence() }
        .filter { token -> token.isNotEmpty() }
        .toList()

    // your code here
}
"#,
    },
    Language {
        slug: "csharp",
        display_name: "C#",
        judge0_id: 51,
        judge0_name_contains: "C#",
        monaco_mode: "csharp",
        file_extension: "cs",
        compiler_options: None,
        starter_code: r#"using System;

public class Program
{
    public static void Main()
    {
        string[] data = (Console.In.ReadToEnd() ?? string.Empty)
            .Split(new[] { ' ', '\t', '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);

        // your code here
    }
}
"#,
    },
    Language {
        slug: "swift",
        display_name: "Swift",
        judge0_id: 83,
        judge0_name_contains: "Swift",
        monaco_mode: "swift",
        file_extension: "swift",
        compiler_options: None,
        starter_code: r#"import Foundation

var input = ""
while let line = readLine(strippingNewline: false) {
    input += line
}

let data = input
    .split(whereSeparator: { $0 == " " || $0 == "\n" || $0 == "\t" || $0 == "\r" })
    .map(String.init)

// your code here
"#,
    },
    Language {
        slug: "dart",
        display_name: "Dart",
        judge0_id: 90,
        judge0_name_contains: "Dart",
        monaco_mode: "dart",
        file_extension: "dart",
        compiler_options: None,
        starter_code: r#"import 'dart:io';

void main() {
  final buffer = StringBuffer();
  String? line;
  while ((line = stdin.readLineSync()) != null) {
    buffer.writeln(line);
  }

  final data = buffer
      .toString()
      .split(RegExp(r'\s+'))
      .where((token) => token.isNotEmpty)
      .toList();

  // your code here
}
"#,
    },
    Language {
        slug: "c",
        display_name: "C",
        judge0_id: 50,
        judge0_name_contains: "C (GCC",
        monaco_mode: "c",
        file_extension: "c",
        compiler_options: None,
        starter_code: r#"#include <stdio.h>

int main(void) {
    /* Read with scanf, for example: int n; scanf("%d", &n); */

    /* your code here */

    return 0;
}
"#,
    },
    Language {
        slug: "cpp",
        display_name: "C++17",
        judge0_id: 54,
        judge0_name_contains: "C++ (GCC",
        monaco_mode: "cpp",
        file_extension: "cpp",
        // GCC 9.2 defaults to gnu++14. Without this flag "C++17" is a lie.
        compiler_options: Some("-std=c++17"),
        starter_code: r#"#include <bits/stdc++.h>

using namespace std;

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    // your code here

    return 0;
}
"#,
    },
    Language {
        slug: "go",
        display_name: "Go",
        judge0_id: 60,
        judge0_name_contains: "Go",
        monaco_mode: "go",
        file_extension: "go",
        compiler_options: None,
        starter_code: r#"package main

import (
	"bufio"
	"os"
)

func main() {
	scanner := bufio.NewScanner(os.Stdin)
	scanner.Buffer(make([]byte, 1024*1024), 1024*1024)
	scanner.Split(bufio.ScanWords)

	data := make([]string, 0)
	for scanner.Scan() {
		data = append(data, scanner.Text())
	}

	writer := bufio.NewWriter(os.Stdout)
	defer writer.Flush()

	// Go rejects unused variables; drop this line once you use data.
	_ = data

	// your code here
}
"#,
    },
    Language {
        slug: "rust",
        display_name: "Rust",
        judge0_id: 73,
        judge0_name_contains: "Rust",
        monaco_mode: "rust",
        file_extension: "rs",
        compiler_options: None,
        starter_code: r#"use std::io::{self, BufWriter, Read, Write};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");
    let data: Vec<&str> = input.split_whitespace().collect();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // your code here

    out.flush().expect("failed to flush stdout");
}
"#,
    },
];

/// Looks up a language by its slug. Case-sensitive: slugs are an exact wire
/// identifier, not user-entered text.
pub fn by_slug(slug: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|language| language.slug == slug)
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test languages`
Expected: PASS, 7 tests.

- [ ] **Step 6: Cross-check the ids against the real judge**

Compare every `judge0_id` above against the output saved in Task 1 Step 8. Correct any that differ. This is a manual check now and an automated one in Task 8.

- [ ] **Step 7: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/mod.rs src/services/practice/mod.rs src/services/practice/languages.rs tests/languages.rs
git commit -m "feat: add language table for the practice arena"
```

---

## Task 4: Entry-point guards for Java and Kotlin

**Files:**
- Modify: `src/services/practice/languages.rs`
- Modify: `tests/languages.rs`

**Interfaces:**
- Consumes: `Language` from Task 3.
- Produces: `pub fn validate_entry_point(language: &Language, source: &str) -> Result<(), String>` — `Err` carries a message written for a learner, which the submit handler in Phase 3 returns as a 400.

- [ ] **Step 1: Write the failing test**

Append to `tests/languages.rs`:

```rust
use slinttech_server::services::practice::languages::validate_entry_point;

fn language(slug: &str) -> &'static languages::Language {
    languages::by_slug(slug).expect("language must be configured")
}

#[test]
fn java_accepts_a_main_class() {
    let source = "public class Main {\n    public static void main(String[] a) {}\n}\n";
    assert!(validate_entry_point(language("java"), source).is_ok());
}

#[test]
fn java_accepts_a_non_public_class_of_any_name() {
    // Judge0 only constrains the *public* class name; `class Solution` compiles.
    let source = "class Solution {\n    public static void main(String[] a) {}\n}\n";
    assert!(validate_entry_point(language("java"), source).is_ok());
}

#[test]
fn java_rejects_a_public_class_that_is_not_main() {
    let source = "public class Solution {\n    public static void main(String[] a) {}\n}\n";
    let error = validate_entry_point(language("java"), source)
        .expect_err("a public class named Solution must be rejected");
    assert!(error.contains("Main"), "message must name the fix: {error}");
    assert!(error.contains("Solution"), "message must name the offender: {error}");
}

#[test]
fn java_accepts_a_public_final_class_named_main() {
    let source = "public final class Main {\n    public static void main(String[] a) {}\n}\n";
    assert!(validate_entry_point(language("java"), source).is_ok());
}

#[test]
fn java_ignores_a_class_name_inside_a_comment() {
    let source = concat!(
        "// public class Solution was the old name\n",
        "public class Main {\n",
        "    public static void main(String[] a) {}\n",
        "}\n"
    );
    assert!(validate_entry_point(language("java"), source).is_ok());
}

#[test]
fn kotlin_accepts_a_top_level_main() {
    assert!(validate_entry_point(language("kotlin"), "fun main() {\n}\n").is_ok());
}

#[test]
fn kotlin_accepts_main_with_arguments() {
    let source = "fun main(args: Array<String>) {\n}\n";
    assert!(validate_entry_point(language("kotlin"), source).is_ok());
}

#[test]
fn kotlin_rejects_a_file_with_no_top_level_main() {
    let source = "class Solution {\n    fun main() {}\n}\n";
    let error = validate_entry_point(language("kotlin"), source)
        .expect_err("an indented main is a member function, not an entry point");
    assert!(error.contains("main"), "message must name the fix: {error}");
}

#[test]
fn other_languages_are_not_constrained() {
    for slug in ["python", "rust", "go", "cpp", "c", "php", "ruby", "swift", "dart"] {
        assert!(
            validate_entry_point(language(slug), "anything at all").is_ok(),
            "{slug} must not be entry-point checked"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test languages`
Expected: FAIL — `cannot find function validate_entry_point`.

- [ ] **Step 3: Implement the guards**

Append to `src/services/practice/languages.rs`:

```rust
use regex::Regex;
use std::sync::LazyLock;

/// Matches a `public class Foo` declaration at the start of a line, allowing the
/// modifiers Java permits between `public` and `class`.
static JAVA_PUBLIC_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*public[ \t]+(?:final[ \t]+|abstract[ \t]+)?class[ \t]+([A-Za-z_$][A-Za-z0-9_$]*)")
        .expect("JAVA_PUBLIC_CLASS is a valid regex")
});

/// Matches a top-level `fun main` — one that starts at column zero. An indented
/// `fun main` is a member function and will not be used as an entry point.
static KOTLIN_TOP_LEVEL_MAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^fun[ \t]+main[ \t]*\(").expect("KOTLIN_TOP_LEVEL_MAIN is a valid regex")
});

/// Rejects submissions whose entry point Judge0 will not find.
///
/// Judge0 writes each submission to a fixed filename, so Java's public class
/// must be `Main` and Kotlin needs a top-level `main`. Left to the compiler,
/// both produce errors that reference a filename the user never chose and give
/// a learner nothing to act on. Catching them here costs one regex and turns
/// the failure into a sentence.
///
/// Returns `Ok(())` for every language without such a constraint.
pub fn validate_entry_point(language: &Language, source: &str) -> Result<(), String> {
    match language.slug {
        "java" => match JAVA_PUBLIC_CLASS.captures(&strip_line_comments(source)) {
            Some(found) if &found[1] != "Main" => Err(format!(
                "Java submissions must declare `public class Main`, but this file declares \
                 `public class {}`. Rename the class to Main, or drop the `public` modifier.",
                &found[1]
            )),
            _ => Ok(()),
        },
        "kotlin" => {
            if KOTLIN_TOP_LEVEL_MAIN.is_match(&strip_line_comments(source)) {
                Ok(())
            } else {
                Err("Kotlin submissions need a top-level `fun main()` starting at the \
                     beginning of a line. A `main` inside a class is a member function and \
                     will not be run."
                    .to_string())
            }
        }
        _ => Ok(()),
    }
}

/// Blanks out `//` line comments so a class name mentioned in a comment is not
/// mistaken for a declaration. Block comments and string literals are not
/// handled: this is a helpful guard, not a parser, and its failure mode is
/// falling through to the compiler's own error.
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test languages`
Expected: PASS, 16 tests.

- [ ] **Step 5: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/practice/languages.rs tests/languages.rs
git commit -m "feat: reject java and kotlin submissions with no usable entry point"
```

---

## Task 5: Verdict aggregation

**Files:**
- Create: `src/services/practice/verdict.rs`
- Modify: `src/services/practice/mod.rs`
- Create: `tests/verdict.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Verdict { Accepted, WrongAnswer, CompilationError, RuntimeError, TimeLimitExceeded, MemoryLimitExceeded, InternalError }` with `pub fn as_str(self) -> &'static str`.
  - `pub struct CaseResult { pub ordinal: i32, pub status_id: i32, pub time_ms: Option<i32>, pub memory_kb: Option<i32>, pub stdout: Option<String>, pub stderr: Option<String>, pub compile_output: Option<String> }`
  - `pub struct JudgeOutcome { pub verdict: Verdict, pub passed_tests: i32, pub total_tests: i32, pub runtime_ms: Option<i32>, pub memory_kb: Option<i32>, pub failed_test_ordinal: Option<i32>, pub compile_output: Option<String>, pub error_message: Option<String> }`
  - `pub fn aggregate(cases: &[CaseResult], memory_limit_kb: i32) -> JudgeOutcome`
  - `pub fn truncate_output(value: Option<String>) -> Option<String>`
  - `pub mod status` — the Judge0 status id constants.
  - Task 7 calls `aggregate`; Phase 3's worker persists `JudgeOutcome`.

- [ ] **Step 1: Write the failing test**

Create `tests/verdict.rs`:

```rust
//! Verdict aggregation is the most rule-dense logic in the feature and has no
//! I/O, so it is tested exhaustively here rather than through the judge client.

use slinttech_server::services::practice::verdict::{
    aggregate, status, truncate_output, CaseResult, Verdict,
};

const MEMORY_LIMIT_KB: i32 = 262_144;

fn case(ordinal: i32, status_id: i32) -> CaseResult {
    CaseResult {
        ordinal,
        status_id,
        time_ms: Some(10),
        memory_kb: Some(1_024),
        stdout: None,
        stderr: None,
        compile_output: None,
    }
}

#[test]
fn all_cases_accepted_is_accepted() {
    let cases = vec![case(1, status::ACCEPTED), case(2, status::ACCEPTED)];
    let outcome = aggregate(&cases, MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::Accepted);
    assert_eq!(outcome.passed_tests, 2);
    assert_eq!(outcome.total_tests, 2);
    assert_eq!(outcome.failed_test_ordinal, None);
}

#[test]
fn a_compilation_error_wins_over_every_other_status() {
    // Compilation is a property of the submission, not of a test case, so it
    // decides the verdict even when it is not the lowest-ordinal failure.
    let mut cases = vec![
        case(1, status::WRONG_ANSWER),
        case(2, status::COMPILATION_ERROR),
    ];
    cases[1].compile_output = Some("error: expected `;`".to_string());

    let outcome = aggregate(&cases, MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::CompilationError);
    assert_eq!(outcome.passed_tests, 0);
    assert_eq!(outcome.failed_test_ordinal, None);
    assert_eq!(outcome.compile_output.as_deref(), Some("error: expected `;`"));
}

#[test]
fn the_lowest_ordinal_failure_decides_the_verdict() {
    let cases = vec![
        case(1, status::ACCEPTED),
        case(3, status::TIME_LIMIT_EXCEEDED),
        case(2, status::WRONG_ANSWER),
    ];

    let outcome = aggregate(&cases, MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::WrongAnswer);
    assert_eq!(outcome.failed_test_ordinal, Some(2));
    assert_eq!(outcome.passed_tests, 1);
    assert_eq!(outcome.total_tests, 3);
}

#[test]
fn cases_are_ordered_by_ordinal_not_by_position() {
    // Judge0 returns results in submission order, but a caller that builds the
    // batch out of order must still get a deterministic verdict.
    let cases = vec![case(9, status::WRONG_ANSWER), case(2, status::RUNTIME_ERROR_NZEC)];
    let outcome = aggregate(&cases, MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::RuntimeError);
    assert_eq!(outcome.failed_test_ordinal, Some(2));
}

#[test]
fn time_limit_exceeded_maps_through() {
    let outcome = aggregate(&[case(1, status::TIME_LIMIT_EXCEEDED)], MEMORY_LIMIT_KB);
    assert_eq!(outcome.verdict, Verdict::TimeLimitExceeded);
}

#[test]
fn every_runtime_error_signal_maps_to_runtime_error() {
    for status_id in [
        status::RUNTIME_ERROR_SIGXFSZ,
        status::RUNTIME_ERROR_SIGFPE,
        status::RUNTIME_ERROR_SIGABRT,
        status::RUNTIME_ERROR_NZEC,
        status::RUNTIME_ERROR_OTHER,
    ] {
        let outcome = aggregate(&[case(1, status_id)], MEMORY_LIMIT_KB);
        assert_eq!(outcome.verdict, Verdict::RuntimeError, "status {status_id}");
    }
}

#[test]
fn a_segfault_well_under_the_memory_cap_is_a_runtime_error() {
    let mut failing = case(1, status::RUNTIME_ERROR_SIGSEGV);
    failing.memory_kb = Some(2_048);

    let outcome = aggregate(&[failing], MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::RuntimeError);
}

#[test]
fn a_segfault_at_the_memory_cap_is_reclassified_as_memory_limit_exceeded() {
    // Judge0 reports an OOM kill as SIGSEGV, so the distinction is inferred.
    let mut failing = case(1, status::RUNTIME_ERROR_SIGSEGV);
    failing.memory_kb = Some(MEMORY_LIMIT_KB);

    let outcome = aggregate(&[failing], MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::MemoryLimitExceeded);
}

#[test]
fn internal_and_exec_format_errors_map_to_internal_error() {
    for status_id in [status::INTERNAL_ERROR, status::EXEC_FORMAT_ERROR] {
        let outcome = aggregate(&[case(1, status_id)], MEMORY_LIMIT_KB);
        assert_eq!(outcome.verdict, Verdict::InternalError, "status {status_id}");
    }
}

#[test]
fn a_still_running_case_is_an_internal_error() {
    // aggregate is only ever called on a terminal batch. Reaching it with a
    // queued case means the poll loop has a bug; failing loudly beats awarding
    // points for an unfinished run.
    for status_id in [status::IN_QUEUE, status::PROCESSING] {
        let outcome = aggregate(&[case(1, status_id)], MEMORY_LIMIT_KB);
        assert_eq!(outcome.verdict, Verdict::InternalError, "status {status_id}");
    }
}

#[test]
fn an_empty_batch_is_an_internal_error() {
    let outcome = aggregate(&[], MEMORY_LIMIT_KB);

    assert_eq!(outcome.verdict, Verdict::InternalError);
    assert_eq!(outcome.total_tests, 0);
    assert_eq!(outcome.passed_tests, 0);
}

#[test]
fn runtime_and_memory_are_the_maxima_across_cases() {
    let mut slow = case(1, status::ACCEPTED);
    slow.time_ms = Some(120);
    slow.memory_kb = Some(4_096);

    let mut fast = case(2, status::ACCEPTED);
    fast.time_ms = Some(15);
    fast.memory_kb = Some(9_000);

    let outcome = aggregate(&[slow, fast], MEMORY_LIMIT_KB);

    assert_eq!(outcome.runtime_ms, Some(120));
    assert_eq!(outcome.memory_kb, Some(9_000));
}

#[test]
fn stderr_from_the_failing_case_becomes_the_error_message() {
    let mut failing = case(2, status::RUNTIME_ERROR_NZEC);
    failing.stderr = Some("IndexError: list index out of range".to_string());

    let outcome = aggregate(&[case(1, status::ACCEPTED), failing], MEMORY_LIMIT_KB);

    assert_eq!(
        outcome.error_message.as_deref(),
        Some("IndexError: list index out of range")
    );
}

#[test]
fn stored_output_is_truncated_to_eight_kilobytes() {
    let huge = "x".repeat(20_000);
    let truncated = truncate_output(Some(huge)).expect("input was Some");

    assert!(truncated.len() < 9_000, "length was {}", truncated.len());
    assert!(truncated.ends_with("output truncated"));
}

#[test]
fn truncation_does_not_split_a_multibyte_character() {
    // A naive slice at byte 8192 panics if it lands mid-character.
    let multibyte = "é".repeat(20_000);
    let truncated = truncate_output(Some(multibyte)).expect("input was Some");

    assert!(truncated.ends_with("output truncated"));
}

#[test]
fn short_output_is_returned_unchanged() {
    assert_eq!(truncate_output(Some("ok".to_string())).as_deref(), Some("ok"));
    assert_eq!(truncate_output(None), None);
}

#[test]
fn verdicts_serialise_to_the_strings_the_database_stores() {
    assert_eq!(Verdict::Accepted.as_str(), "accepted");
    assert_eq!(Verdict::WrongAnswer.as_str(), "wrong_answer");
    assert_eq!(Verdict::CompilationError.as_str(), "compilation_error");
    assert_eq!(Verdict::RuntimeError.as_str(), "runtime_error");
    assert_eq!(Verdict::TimeLimitExceeded.as_str(), "time_limit_exceeded");
    assert_eq!(Verdict::MemoryLimitExceeded.as_str(), "memory_limit_exceeded");
    assert_eq!(Verdict::InternalError.as_str(), "internal_error");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test verdict`
Expected: FAIL — `unresolved import ...::practice::verdict`.

- [ ] **Step 3: Implement the aggregation**

Create `src/services/practice/verdict.rs`:

```rust
//! Folds Judge0's per-test-case results into the single verdict a submission
//! carries. Pure: no I/O, no database, no clock.

use serde::{Deserialize, Serialize};

/// Judge0 status ids, from `GET /statuses` on Judge0 CE 1.13.x.
pub mod status {
    pub const IN_QUEUE: i32 = 1;
    pub const PROCESSING: i32 = 2;
    pub const ACCEPTED: i32 = 3;
    pub const WRONG_ANSWER: i32 = 4;
    pub const TIME_LIMIT_EXCEEDED: i32 = 5;
    pub const COMPILATION_ERROR: i32 = 6;
    pub const RUNTIME_ERROR_SIGSEGV: i32 = 7;
    pub const RUNTIME_ERROR_SIGXFSZ: i32 = 8;
    pub const RUNTIME_ERROR_SIGFPE: i32 = 9;
    pub const RUNTIME_ERROR_SIGABRT: i32 = 10;
    pub const RUNTIME_ERROR_NZEC: i32 = 11;
    pub const RUNTIME_ERROR_OTHER: i32 = 12;
    pub const INTERNAL_ERROR: i32 = 13;
    pub const EXEC_FORMAT_ERROR: i32 = 14;
}

/// Cap on stored judge output, so a program that prints megabytes cannot bloat
/// a database row.
pub const MAX_STORED_OUTPUT_BYTES: usize = 8 * 1024;

/// Fraction of the memory limit at or above which a SIGSEGV is treated as an
/// out-of-memory kill rather than a genuine segfault. Judge0 does not report
/// OOM separately, so the distinction has to be inferred from usage.
const MEMORY_LIMIT_INFERENCE_RATIO: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Accepted,
    WrongAnswer,
    CompilationError,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    InternalError,
}

impl Verdict {
    /// The exact string stored in `problem_submissions.verdict`.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Accepted => "accepted",
            Verdict::WrongAnswer => "wrong_answer",
            Verdict::CompilationError => "compilation_error",
            Verdict::RuntimeError => "runtime_error",
            Verdict::TimeLimitExceeded => "time_limit_exceeded",
            Verdict::MemoryLimitExceeded => "memory_limit_exceeded",
            Verdict::InternalError => "internal_error",
        }
    }
}

/// One test case's result, as reported by Judge0 and decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    pub ordinal: i32,
    pub status_id: i32,
    pub time_ms: Option<i32>,
    pub memory_kb: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub compile_output: Option<String>,
}

/// The whole submission's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeOutcome {
    pub verdict: Verdict,
    pub passed_tests: i32,
    pub total_tests: i32,
    pub runtime_ms: Option<i32>,
    pub memory_kb: Option<i32>,
    /// Index of the first failing case. Only the index is ever exposed for
    /// hidden cases — never their input, expected output, or actual output.
    pub failed_test_ordinal: Option<i32>,
    pub compile_output: Option<String>,
    pub error_message: Option<String>,
}

/// Folds per-case results into one verdict.
///
/// The rules, in order:
/// 1. Any compilation error wins outright — compilation is a property of the
///    submission, not of a test case.
/// 2. Otherwise the lowest-ordinal case that is not Accepted decides.
/// 3. All cases Accepted is Accepted.
///
/// `memory_limit_kb` is the limit the batch was run under, needed because
/// Judge0 reports an out-of-memory kill as SIGSEGV.
pub fn aggregate(cases: &[CaseResult], memory_limit_kb: i32) -> JudgeOutcome {
    let total_tests = cases.len() as i32;
    let runtime_ms = cases.iter().filter_map(|case| case.time_ms).max();
    let memory_kb = cases.iter().filter_map(|case| case.memory_kb).max();

    if cases.is_empty() {
        return JudgeOutcome {
            verdict: Verdict::InternalError,
            passed_tests: 0,
            total_tests: 0,
            runtime_ms: None,
            memory_kb: None,
            failed_test_ordinal: None,
            compile_output: None,
            error_message: Some("The judge returned no results.".to_string()),
        };
    }

    if let Some(failed) = cases
        .iter()
        .find(|case| case.status_id == status::COMPILATION_ERROR)
    {
        return JudgeOutcome {
            verdict: Verdict::CompilationError,
            passed_tests: 0,
            total_tests,
            runtime_ms,
            memory_kb,
            failed_test_ordinal: None,
            compile_output: truncate_output(failed.compile_output.clone()),
            error_message: None,
        };
    }

    let passed_tests = cases
        .iter()
        .filter(|case| case.status_id == status::ACCEPTED)
        .count() as i32;

    let first_failure = cases
        .iter()
        .filter(|case| case.status_id != status::ACCEPTED)
        .min_by_key(|case| case.ordinal);

    match first_failure {
        None => JudgeOutcome {
            verdict: Verdict::Accepted,
            passed_tests,
            total_tests,
            runtime_ms,
            memory_kb,
            failed_test_ordinal: None,
            compile_output: None,
            error_message: None,
        },
        Some(failed) => JudgeOutcome {
            verdict: verdict_for(failed, memory_limit_kb),
            passed_tests,
            total_tests,
            runtime_ms,
            memory_kb,
            failed_test_ordinal: Some(failed.ordinal),
            compile_output: None,
            error_message: truncate_output(failed.stderr.clone()),
        },
    }
}

fn verdict_for(case: &CaseResult, memory_limit_kb: i32) -> Verdict {
    match case.status_id {
        status::WRONG_ANSWER => Verdict::WrongAnswer,
        status::TIME_LIMIT_EXCEEDED => Verdict::TimeLimitExceeded,
        status::RUNTIME_ERROR_SIGSEGV if hit_memory_limit(case, memory_limit_kb) => {
            Verdict::MemoryLimitExceeded
        }
        status::RUNTIME_ERROR_SIGSEGV
        | status::RUNTIME_ERROR_SIGXFSZ
        | status::RUNTIME_ERROR_SIGFPE
        | status::RUNTIME_ERROR_SIGABRT
        | status::RUNTIME_ERROR_NZEC
        | status::RUNTIME_ERROR_OTHER => Verdict::RuntimeError,
        _ => Verdict::InternalError,
    }
}

fn hit_memory_limit(case: &CaseResult, memory_limit_kb: i32) -> bool {
    match case.memory_kb {
        Some(used) if memory_limit_kb > 0 => {
            f64::from(used) >= f64::from(memory_limit_kb) * MEMORY_LIMIT_INFERENCE_RATIO
        }
        _ => false,
    }
}

/// Caps stored output at [`MAX_STORED_OUTPUT_BYTES`], never splitting a
/// multi-byte character.
pub fn truncate_output(value: Option<String>) -> Option<String> {
    value.map(|text| {
        if text.len() <= MAX_STORED_OUTPUT_BYTES {
            return text;
        }

        let mut cut = MAX_STORED_OUTPUT_BYTES;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }

        format!("{}\n… output truncated", &text[..cut])
    })
}
```

- [ ] **Step 4: Register the module**

In `src/services/practice/mod.rs`, add:

```rust
pub mod verdict;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test verdict`
Expected: PASS, 17 tests.

- [ ] **Step 6: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/practice/mod.rs src/services/practice/verdict.rs tests/verdict.rs
git commit -m "feat: aggregate judge0 case results into a submission verdict"
```

---

## Task 6: Judge0 client and batch submission

**Files:**
- Create: `src/services/practice/judge.rs`
- Modify: `src/services/practice/mod.rs`
- Create: `tests/judge_client.rs`

**Interfaces:**
- Consumes: `Config` (Task 2), `Language` (Task 3), `CaseResult` and `status` (Task 5).
- Produces:
  - `pub enum JudgeError { Transport, Response { status, body }, Decode(String), Deadline(Duration) }`
  - `pub struct ExecutionLimits { pub cpu_time_limit_ms: i32, pub memory_limit_kb: i32 }` with `Default`.
  - `pub struct TestCaseInput { pub ordinal: i32, pub stdin: String, pub expected_output: String }`
  - `pub struct Judge0Client` with `pub fn new(config: &Config) -> Result<Self, JudgeError>` and `pub async fn submit_batch(&self, language: &Language, source_code: &str, cases: &[TestCaseInput], limits: ExecutionLimits) -> Result<Vec<String>, JudgeError>`.
  - Task 7 adds `run_batch` to the same type; Task 8 adds `fetch_languages`.

- [ ] **Step 1: Write the failing test**

Create `tests/judge_client.rs`:

```rust
//! The judge client is tested against a mock HTTP server, so the unit suite
//! needs no running Judge0. Live-judge behaviour is covered by `judge_live.rs`.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::Value;
use slinttech_server::config::Config;
use slinttech_server::services::practice::judge::{
    ExecutionLimits, Judge0Client, JudgeError, TestCaseInput,
};
use slinttech_server::services::practice::languages;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Builds a Config pointed at the mock server. Only the judge fields matter.
fn config_for(server: &MockServer, token: Option<&str>) -> Config {
    Config {
        database_url: "postgres://localhost/unused".to_string(),
        jwt_secret: "unused".to_string(),
        jwt_expiration: 604_800,
        server_host: "127.0.0.1".to_string(),
        server_port: 8698,
        app_url: "http://localhost:8698".to_string(),
        cors_allowed_origins: vec!["*".to_string()],
        paystack_secret_key: None,
        cloudinary_cloud_name: None,
        cloudinary_api_key: None,
        cloudinary_api_secret: None,
        judge0_url: server.uri().trim_end_matches('/').to_string(),
        judge0_auth_token: token.map(str::to_string),
        judge_max_inflight: 4,
        judge_worker_tick_ms: 250,
        judge_max_attempts: 3,
        judge_stale_claim_seconds: 300,
        judge_batch_deadline_seconds: 5,
        judge_batch_poll_start_ms: 10,
        judge_batch_poll_max_ms: 40,
        practice_max_source_bytes: 65_536,
    }
}

fn cases() -> Vec<TestCaseInput> {
    vec![
        TestCaseInput {
            ordinal: 1,
            stdin: "1 2\n".to_string(),
            expected_output: "3\n".to_string(),
        },
        TestCaseInput {
            ordinal: 2,
            stdin: "5 7\n".to_string(),
            expected_output: "12\n".to_string(),
        },
    ]
}

fn decode(value: &Value, key: &str) -> String {
    let encoded = value[key].as_str().expect("field must be a string");
    String::from_utf8(BASE64.decode(encoded).expect("field must be base64"))
        .expect("field must be utf-8")
}

#[tokio::test]
async fn submit_batch_sends_one_entry_per_test_case() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([
            { "token": "token-one" },
            { "token": "token-two" },
        ])))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    let tokens = client
        .submit_batch(python, "print(1)", &cases(), ExecutionLimits::default())
        .await
        .expect("submit succeeds");

    assert_eq!(tokens, vec!["token-one".to_string(), "token-two".to_string()]);

    let requests = server.received_requests().await.expect("requests recorded");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("body is json");
    let submissions = body["submissions"].as_array().expect("submissions array");

    assert_eq!(submissions.len(), 2);
    assert_eq!(decode(&submissions[0], "stdin"), "1 2\n");
    assert_eq!(decode(&submissions[0], "expected_output"), "3\n");
    assert_eq!(decode(&submissions[1], "stdin"), "5 7\n");
    assert_eq!(decode(&submissions[0], "source_code"), "print(1)");
    assert_eq!(submissions[0]["language_id"], 71);
}

#[tokio::test]
async fn every_submission_disables_networking() {
    // The single most important guard in the feature. If this test ever fails,
    // untrusted code can reach the internet from inside the sandbox.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let rust = languages::by_slug("rust").expect("rust is configured");

    client
        .submit_batch(rust, "fn main() {}", &cases()[..1], ExecutionLimits::default())
        .await
        .expect("submit succeeds");

    let requests = server.received_requests().await.expect("requests recorded");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("body is json");

    assert_eq!(body["submissions"][0]["enable_network"], false);
}

#[tokio::test]
async fn submissions_carry_the_resource_limits() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");
    let limits = ExecutionLimits {
        cpu_time_limit_ms: 3_000,
        memory_limit_kb: 131_072,
    };

    client
        .submit_batch(python, "print(1)", &cases()[..1], limits)
        .await
        .expect("submit succeeds");

    let requests = server.received_requests().await.expect("requests recorded");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("body is json");
    let submission = &body["submissions"][0];

    assert_eq!(submission["cpu_time_limit"], 3.0);
    assert_eq!(submission["cpu_extra_time"], 0.5);
    // Wall time is double the CPU limit so a sleeping program, which burns no
    // CPU, is still killed.
    assert_eq!(submission["wall_time_limit"], 6.0);
    assert_eq!(submission["memory_limit"], 131_072);
    assert_eq!(submission["max_processes_and_or_threads"], 60);
    assert_eq!(submission["max_file_size"], 1_024);
    assert_eq!(submission["number_of_runs"], 1);
    assert_eq!(submission["redirect_stderr_to_stdout"], false);
}

#[tokio::test]
async fn cpp_submissions_request_the_cpp17_standard() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let cpp = languages::by_slug("cpp").expect("cpp is configured");

    client
        .submit_batch(cpp, "int main(){}", &cases()[..1], ExecutionLimits::default())
        .await
        .expect("submit succeeds");

    let requests = server.received_requests().await.expect("requests recorded");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("body is json");

    assert_eq!(body["submissions"][0]["compiler_options"], "-std=c++17");
}

#[tokio::test]
async fn languages_without_compiler_options_omit_the_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    client
        .submit_batch(python, "print(1)", &cases()[..1], ExecutionLimits::default())
        .await
        .expect("submit succeeds");

    let requests = server.received_requests().await.expect("requests recorded");
    let body: Value = serde_json::from_slice(&requests[0].body).expect("body is json");

    assert!(body["submissions"][0].get("compiler_options").is_none());
}

#[tokio::test]
async fn the_auth_token_is_sent_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .and(header("X-Auth-Token", "secret-token"))
        .and(query_param("base64_encoded", "true"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    let client =
        Judge0Client::new(&config_for(&server, Some("secret-token"))).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    // The mock only matches when the header and query string are present, so a
    // success here proves both were sent.
    client
        .submit_batch(python, "print(1)", &cases()[..1], ExecutionLimits::default())
        .await
        .expect("submit succeeds");
}

#[tokio::test]
async fn a_judge_error_response_is_surfaced_with_its_status_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(503).set_body_string("queue is full"))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    let error = client
        .submit_batch(python, "print(1)", &cases()[..1], ExecutionLimits::default())
        .await
        .expect_err("a 503 must surface as an error");

    match error {
        JudgeError::Response { status, body } => {
            assert_eq!(status, 503);
            assert!(body.contains("queue is full"));
        }
        other => panic!("expected a Response error, got {other:?}"),
    }
}

#[tokio::test]
async fn an_empty_case_list_is_rejected_without_calling_the_judge() {
    let server = MockServer::start().await;
    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    let error = client
        .submit_batch(python, "print(1)", &[], ExecutionLimits::default())
        .await
        .expect_err("an empty batch is a caller bug");

    assert!(matches!(error, JudgeError::Decode(_)));
    assert!(server.received_requests().await.expect("recorded").is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test judge_client`
Expected: FAIL — `unresolved import ...::practice::judge`.

- [ ] **Step 3: Implement the client**

Create `src/services/practice/judge.rs`:

```rust
//! HTTP client for the Judge0 execution engine.
//!
//! Deliberately domain-free: it takes source, a language and a list of
//! (stdin, expected output) pairs, and returns raw per-case results. Folding
//! those into a verdict is `super::verdict`'s job, and persisting them is the
//! worker's. That separation is what lets this be tested against a mock server
//! with no database in sight.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Config;
use crate::services::practice::languages::Language;

/// Extra CPU grace Judge0 allows before killing a run outright.
const CPU_EXTRA_TIME_SECONDS: f64 = 0.5;

/// Wall-clock limit as a multiple of the CPU limit. A program that sleeps or
/// blocks on I/O burns no CPU, so a CPU limit alone would never stop it.
const WALL_TIME_MULTIPLIER: f64 = 2.0;

/// Fork-bomb guard. Not 1: the JVM and the Go runtime legitimately spawn
/// threads, and too low a value fails perfectly valid submissions.
const MAX_PROCESSES_AND_OR_THREADS: i32 = 60;

/// Stops a submission filling the disk.
const MAX_FILE_SIZE_KB: i32 = 1_024;

/// How long a single HTTP call to Judge0 may take.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("judge is unreachable: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("judge returned HTTP {status}: {body}")]
    Response { status: u16, body: String },

    #[error("judge response was not understood: {0}")]
    Decode(String),

    #[error("judge did not finish within {0:?}")]
    Deadline(Duration),
}

/// Per-execution resource limits, taken from the problem being judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub cpu_time_limit_ms: i32,
    pub memory_limit_kb: i32,
}

impl ExecutionLimits {
    pub const DEFAULT_CPU_TIME_LIMIT_MS: i32 = 2_000;
    pub const DEFAULT_MEMORY_LIMIT_KB: i32 = 262_144;
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            cpu_time_limit_ms: Self::DEFAULT_CPU_TIME_LIMIT_MS,
            memory_limit_kb: Self::DEFAULT_MEMORY_LIMIT_KB,
        }
    }
}

/// One test case to run the submission against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseInput {
    pub ordinal: i32,
    pub stdin: String,
    pub expected_output: String,
}

/// One entry in a Judge0 batch. Field names are Judge0's wire format.
#[derive(Debug, Serialize)]
struct Judge0Submission {
    source_code: String,
    language_id: i32,
    stdin: String,
    expected_output: String,
    cpu_time_limit: f64,
    cpu_extra_time: f64,
    wall_time_limit: f64,
    memory_limit: i32,
    max_processes_and_or_threads: i32,
    max_file_size: i32,
    number_of_runs: i32,
    redirect_stderr_to_stdout: bool,
    /// Always false. Also pinned off globally in judge0.conf, so this is the
    /// inner of two independent guards.
    enable_network: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_options: Option<String>,
}

#[derive(Debug, Serialize)]
struct Judge0BatchRequest {
    submissions: Vec<Judge0Submission>,
}

#[derive(Debug, Deserialize)]
struct Judge0Token {
    token: String,
}

pub struct Judge0Client {
    http: reqwest::Client,
    base_url: String,
    auth_token: Option<String>,
    pub(crate) poll_start: Duration,
    pub(crate) poll_max: Duration,
    pub(crate) deadline: Duration,
}

impl Judge0Client {
    pub fn new(config: &Config) -> Result<Self, JudgeError> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?,
            base_url: config.judge0_url.trim_end_matches('/').to_string(),
            auth_token: config.judge0_auth_token.clone(),
            poll_start: Duration::from_millis(config.judge_batch_poll_start_ms),
            poll_max: Duration::from_millis(config.judge_batch_poll_max_ms),
            deadline: Duration::from_secs(config.judge_batch_deadline_seconds),
        })
    }

    /// Starts a request to Judge0, attaching the auth header when configured.
    pub(crate) fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self.http.request(method, format!("{}{}", self.base_url, path));
        match &self.auth_token {
            Some(token) => builder.header("X-Auth-Token", token),
            None => builder,
        }
    }

    /// Queues one batch — one entry per test case — and returns its tokens in
    /// the same order as `cases`.
    pub async fn submit_batch(
        &self,
        language: &Language,
        source_code: &str,
        cases: &[TestCaseInput],
        limits: ExecutionLimits,
    ) -> Result<Vec<String>, JudgeError> {
        if cases.is_empty() {
            return Err(JudgeError::Decode(
                "cannot judge a submission with no test cases".to_string(),
            ));
        }

        let body = Judge0BatchRequest {
            submissions: cases
                .iter()
                .map(|case| build_submission(language, source_code, case, limits))
                .collect(),
        };

        let response = self
            .request(reqwest::Method::POST, "/submissions/batch")
            .query(&[("base64_encoded", "true"), ("wait", "false")])
            .json(&body)
            .send()
            .await?;

        let response = check_status(response).await?;
        let tokens: Vec<Judge0Token> = response
            .json()
            .await
            .map_err(|error| JudgeError::Decode(error.to_string()))?;

        if tokens.len() != cases.len() {
            return Err(JudgeError::Decode(format!(
                "judge returned {} tokens for {} test cases",
                tokens.len(),
                cases.len()
            )));
        }

        Ok(tokens.into_iter().map(|entry| entry.token).collect())
    }
}

fn build_submission(
    language: &Language,
    source_code: &str,
    case: &TestCaseInput,
    limits: ExecutionLimits,
) -> Judge0Submission {
    let cpu_time_limit = f64::from(limits.cpu_time_limit_ms) / 1_000.0;

    Judge0Submission {
        source_code: BASE64.encode(source_code),
        language_id: language.judge0_id,
        stdin: BASE64.encode(&case.stdin),
        expected_output: BASE64.encode(&case.expected_output),
        cpu_time_limit,
        cpu_extra_time: CPU_EXTRA_TIME_SECONDS,
        wall_time_limit: cpu_time_limit * WALL_TIME_MULTIPLIER,
        memory_limit: limits.memory_limit_kb,
        max_processes_and_or_threads: MAX_PROCESSES_AND_OR_THREADS,
        max_file_size: MAX_FILE_SIZE_KB,
        number_of_runs: 1,
        redirect_stderr_to_stdout: false,
        enable_network: false,
        compiler_options: language.compiler_options.map(str::to_string),
    }
}

/// Converts a non-2xx response into a `JudgeError::Response` carrying the body,
/// which is where Judge0 puts the reason.
pub(crate) async fn check_status(
    response: reqwest::Response,
) -> Result<reqwest::Response, JudgeError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    Err(JudgeError::Response {
        status: status.as_u16(),
        body,
    })
}
```

- [ ] **Step 4: Register the module**

In `src/services/practice/mod.rs`, add:

```rust
pub mod judge;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test judge_client`
Expected: PASS, 8 tests.

- [ ] **Step 6: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/practice/mod.rs src/services/practice/judge.rs tests/judge_client.rs
git commit -m "feat: add judge0 client with batch submission"
```

---

## Task 7: Batch polling and result decoding

**Files:**
- Modify: `src/services/practice/judge.rs`
- Modify: `tests/judge_client.rs`

**Interfaces:**
- Consumes: `submit_batch` (Task 6), `CaseResult` and `status` (Task 5).
- Produces: `pub async fn run_batch(&self, language: &Language, source_code: &str, cases: &[TestCaseInput], limits: ExecutionLimits) -> Result<Vec<CaseResult>, JudgeError>` — results carry the ordinals from `cases`, times in milliseconds, and base64-decoded output. Phase 3's worker calls this and passes the result to `verdict::aggregate`.

- [ ] **Step 1: Write the failing test**

Append to `tests/judge_client.rs`:

```rust
use slinttech_server::services::practice::verdict::status;

/// Judge0 base64-encodes stdout, stderr and compile_output, but not the numeric
/// fields. This mirrors that on the way out.
fn judge0_result(status_id: i32, stdout: &str, time: &str, memory: i64) -> serde_json::Value {
    serde_json::json!({
        "token": "t",
        "status_id": status_id,
        "stdout": BASE64.encode(stdout),
        "stderr": Value::Null,
        "compile_output": Value::Null,
        "time": time,
        "memory": memory,
    })
}

#[tokio::test]
async fn run_batch_polls_until_every_case_is_terminal() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([
            { "token": "token-one" },
            { "token": "token-two" },
        ])))
        .mount(&server)
        .await;

    // First poll: still running. wiremock matches the most recently mounted
    // eligible mock first, so mounting the terminal response second makes this
    // a two-step sequence.
    Mock::given(method("GET"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "submissions": [
                { "token": "token-one", "status_id": status::PROCESSING },
                { "token": "token-two", "status_id": status::IN_QUEUE },
            ]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "submissions": [
                judge0_result(status::ACCEPTED, "3\n", "0.012", 3_200),
                judge0_result(status::WRONG_ANSWER, "13\n", "0.008", 3_100),
            ]
        })))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    let results = client
        .run_batch(python, "print(1)", &cases(), ExecutionLimits::default())
        .await
        .expect("run succeeds");

    assert_eq!(results.len(), 2);

    // Ordinals come from the caller's test cases, not from Judge0.
    assert_eq!(results[0].ordinal, 1);
    assert_eq!(results[1].ordinal, 2);

    assert_eq!(results[0].status_id, status::ACCEPTED);
    assert_eq!(results[1].status_id, status::WRONG_ANSWER);

    // Judge0 reports seconds as a string; we store milliseconds.
    assert_eq!(results[0].time_ms, Some(12));
    assert_eq!(results[1].time_ms, Some(8));
    assert_eq!(results[0].memory_kb, Some(3_200));

    assert_eq!(results[0].stdout.as_deref(), Some("3\n"));
    assert_eq!(results[1].stdout.as_deref(), Some("13\n"));
}

#[tokio::test]
async fn run_batch_gives_up_at_the_deadline() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    // Never becomes terminal.
    Mock::given(method("GET"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "submissions": [{ "token": "t", "status_id": status::PROCESSING }]
        })))
        .mount(&server)
        .await;

    let mut config = config_for(&server, None);
    config.judge_batch_deadline_seconds = 1;

    let client = Judge0Client::new(&config).expect("client builds");
    let python = languages::by_slug("python").expect("python is configured");

    let error = client
        .run_batch(python, "print(1)", &cases()[..1], ExecutionLimits::default())
        .await
        .expect_err("a batch that never finishes must time out");

    assert!(matches!(error, JudgeError::Deadline(_)), "got {error:?}");
}

#[tokio::test]
async fn run_batch_decodes_compile_output_and_stderr() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([{ "token": "t" }])))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/submissions/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "submissions": [{
                "token": "t",
                "status_id": status::COMPILATION_ERROR,
                "stdout": Value::Null,
                "stderr": BASE64.encode("boom"),
                "compile_output": BASE64.encode("error: expected `;`"),
                "time": Value::Null,
                "memory": Value::Null,
            }]
        })))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let rust = languages::by_slug("rust").expect("rust is configured");

    let results = client
        .run_batch(rust, "fn main() {", &cases()[..1], ExecutionLimits::default())
        .await
        .expect("run succeeds");

    assert_eq!(results[0].compile_output.as_deref(), Some("error: expected `;`"));
    assert_eq!(results[0].stderr.as_deref(), Some("boom"));
    assert_eq!(results[0].stdout, None);
    assert_eq!(results[0].time_ms, None);
    assert_eq!(results[0].memory_kb, None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test judge_client`
Expected: FAIL — `no method named run_batch`.

- [ ] **Step 3: Implement polling and decoding**

Append to `src/services/practice/judge.rs`:

```rust
use crate::services::practice::verdict::{status, CaseResult};
use std::time::Instant;

/// One case's raw result as Judge0 reports it.
#[derive(Debug, Deserialize)]
struct Judge0Result {
    #[serde(default)]
    status_id: Option<i32>,
    #[serde(default)]
    stdout: Option<String>,
    #[serde(default)]
    stderr: Option<String>,
    #[serde(default)]
    compile_output: Option<String>,
    /// Seconds, as a string — for example "0.012".
    #[serde(default)]
    time: Option<String>,
    /// Kilobytes.
    #[serde(default)]
    memory: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Judge0BatchResponse {
    submissions: Vec<Judge0Result>,
}

/// Fields we ask Judge0 for. Requesting explicitly keeps the response small and
/// stops an upgrade from silently adding payload we then store.
const RESULT_FIELDS: &str = "token,status_id,stdout,stderr,compile_output,time,memory";

impl Judge0Client {
    /// Runs a submission against every case and returns the decoded results,
    /// ordered to match `cases`.
    ///
    /// Submits one batch, then polls with exponential backoff until every case
    /// is terminal or the deadline passes. A deadline is not a verdict: the
    /// caller decides whether to retry, and no points are ever awarded for a
    /// batch that did not finish.
    pub async fn run_batch(
        &self,
        language: &Language,
        source_code: &str,
        cases: &[TestCaseInput],
        limits: ExecutionLimits,
    ) -> Result<Vec<CaseResult>, JudgeError> {
        let tokens = self
            .submit_batch(language, source_code, cases, limits)
            .await?;

        let started = Instant::now();
        let mut wait = self.poll_start;

        loop {
            let results = self.fetch_batch(&tokens).await?;

            if results.len() != cases.len() {
                return Err(JudgeError::Decode(format!(
                    "judge returned {} results for {} test cases",
                    results.len(),
                    cases.len()
                )));
            }

            if results.iter().all(is_terminal) {
                return Ok(cases
                    .iter()
                    .zip(results)
                    .map(|(case, result)| decode_result(case.ordinal, result))
                    .collect());
            }

            if started.elapsed() >= self.deadline {
                return Err(JudgeError::Deadline(self.deadline));
            }

            tokio::time::sleep(wait).await;
            wait = (wait * 2).min(self.poll_max);
        }
    }

    async fn fetch_batch(&self, tokens: &[String]) -> Result<Vec<Judge0Result>, JudgeError> {
        let response = self
            .request(reqwest::Method::GET, "/submissions/batch")
            .query(&[
                ("tokens", tokens.join(",").as_str()),
                ("base64_encoded", "true"),
                ("fields", RESULT_FIELDS),
            ])
            .send()
            .await?;

        let response = check_status(response).await?;
        let batch: Judge0BatchResponse = response
            .json()
            .await
            .map_err(|error| JudgeError::Decode(error.to_string()))?;

        Ok(batch.submissions)
    }
}

/// A case is terminal once Judge0 has stopped working on it. Anything at or
/// below `PROCESSING`, including a missing status, is still in flight.
fn is_terminal(result: &Judge0Result) -> bool {
    matches!(result.status_id, Some(id) if id > status::PROCESSING)
}

fn decode_result(ordinal: i32, result: Judge0Result) -> CaseResult {
    CaseResult {
        ordinal,
        // A terminal result always carries a status; defaulting to
        // INTERNAL_ERROR keeps a malformed response from reading as Accepted.
        status_id: result.status_id.unwrap_or(status::INTERNAL_ERROR),
        time_ms: seconds_to_millis(result.time.as_deref()),
        memory_kb: result.memory.map(|kilobytes| kilobytes.round() as i32),
        stdout: decode_base64(result.stdout.as_deref()),
        stderr: decode_base64(result.stderr.as_deref()),
        compile_output: decode_base64(result.compile_output.as_deref()),
    }
}

fn decode_base64(value: Option<&str>) -> Option<String> {
    let encoded = value?.trim();
    if encoded.is_empty() {
        return None;
    }

    // Undecodable output is not worth failing a submission over: report it
    // rather than turning a Wrong Answer into an internal error.
    let bytes = BASE64.decode(encoded).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn seconds_to_millis(value: Option<&str>) -> Option<i32> {
    let seconds: f64 = value?.trim().parse().ok()?;
    Some((seconds * 1_000.0).round() as i32)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test judge_client`
Expected: PASS, 11 tests.

- [ ] **Step 5: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/practice/judge.rs tests/judge_client.rs
git commit -m "feat: poll judge0 batches to completion and decode results"
```

---

## Task 8: Boot-time language validation

**Files:**
- Modify: `src/services/practice/judge.rs`
- Modify: `src/main.rs`
- Modify: `tests/judge_client.rs`

**Interfaces:**
- Consumes: `Judge0Client` (Task 6), `LANGUAGES` (Task 3).
- Produces:
  - `pub struct LanguageAvailability { pub available: Vec<&'static str>, pub unavailable: Vec<(&'static str, String)> }`
  - `pub async fn validate_languages(&self) -> Result<LanguageAvailability, JudgeError>` on `Judge0Client`.
  - Phase 2's `GET /api/practice/languages` serves only `available`; Phase 3's submit handler rejects anything outside it.

- [ ] **Step 1: Write the failing test**

Append to `tests/judge_client.rs`:

```rust
use slinttech_server::services::practice::languages::LANGUAGES;

/// A Judge0 `/languages` payload that matches our configured table exactly.
fn matching_language_payload() -> Value {
    Value::Array(
        LANGUAGES
            .iter()
            .map(|language| {
                serde_json::json!({
                    "id": language.judge0_id,
                    "name": format!("{} (1.0.0)", language.judge0_name_contains),
                })
            })
            .collect(),
    )
}

#[tokio::test]
async fn every_configured_language_is_available_when_the_judge_agrees() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/languages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(matching_language_payload()))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let availability = client.validate_languages().await.expect("validation runs");

    assert_eq!(availability.available.len(), 14);
    assert!(availability.unavailable.is_empty());
}

#[tokio::test]
async fn a_language_missing_from_the_judge_is_marked_unavailable() {
    let server = MockServer::start().await;
    let mut payload = matching_language_payload();
    payload.as_array_mut().expect("array").retain(|language| {
        language["id"] != 73 // Rust
    });

    Mock::given(method("GET"))
        .and(path("/languages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let availability = client.validate_languages().await.expect("validation runs");

    assert_eq!(availability.available.len(), 13);
    assert!(!availability.available.contains(&"rust"));
    assert_eq!(availability.unavailable.len(), 1);
    assert_eq!(availability.unavailable[0].0, "rust");
    assert!(availability.unavailable[0].1.contains("not offered"));
}

#[tokio::test]
async fn an_id_pointing_at_a_different_compiler_is_marked_unavailable() {
    // This is the failure a Judge0 upgrade causes: the id still exists but now
    // means something else. Judging PHP against a Ruby compiler is the kind of
    // bug that costs a day to find, so the name is checked too.
    let server = MockServer::start().await;
    let mut payload = matching_language_payload();
    for language in payload.as_array_mut().expect("array") {
        if language["id"] == 68 {
            language["name"] = Value::String("Ruby (2.7.0)".to_string());
        }
    }

    Mock::given(method("GET"))
        .and(path("/languages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let availability = client.validate_languages().await.expect("validation runs");

    assert!(!availability.available.contains(&"php"));
    let (slug, reason) = availability
        .unavailable
        .iter()
        .find(|(slug, _)| *slug == "php")
        .expect("php must be reported unavailable");
    assert_eq!(*slug, "php");
    assert!(reason.contains("Ruby"), "reason must name what was found: {reason}");
}

#[tokio::test]
async fn an_unreachable_judge_surfaces_as_an_error_not_an_empty_list() {
    // Booting with zero available languages because the judge was briefly down
    // would silently disable the whole arena.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/languages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = Judge0Client::new(&config_for(&server, None)).expect("client builds");
    let error = client
        .validate_languages()
        .await
        .expect_err("a 500 must not read as an empty language list");

    assert!(matches!(error, JudgeError::Response { status: 500, .. }));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test judge_client`
Expected: FAIL — `no method named validate_languages`.

- [ ] **Step 3: Implement validation**

Append to `src/services/practice/judge.rs`:

```rust
use crate::services::practice::languages::LANGUAGES;

#[derive(Debug, Deserialize)]
struct Judge0Language {
    id: i32,
    name: String,
}

/// Which configured languages this Judge0 instance can actually run.
#[derive(Debug, Clone, Default)]
pub struct LanguageAvailability {
    pub available: Vec<&'static str>,
    /// Slug and the reason it cannot be offered.
    pub unavailable: Vec<(&'static str, String)>,
}

impl Judge0Client {
    /// Checks every configured language against the judge's own list.
    ///
    /// Judge0's numeric ids shift between releases, so a stale id either
    /// disappears or — worse — starts pointing at a different compiler. Both
    /// are caught here, at boot, rather than by a user receiving nonsense
    /// verdicts. An unreachable judge is an error, not an empty list.
    pub async fn validate_languages(&self) -> Result<LanguageAvailability, JudgeError> {
        let response = self.request(reqwest::Method::GET, "/languages").send().await?;
        let response = check_status(response).await?;
        let offered: Vec<Judge0Language> = response
            .json()
            .await
            .map_err(|error| JudgeError::Decode(error.to_string()))?;

        let mut availability = LanguageAvailability::default();

        for language in LANGUAGES {
            match offered.iter().find(|entry| entry.id == language.judge0_id) {
                None => availability.unavailable.push((
                    language.slug,
                    format!(
                        "judge0 language id {} is not offered by this instance",
                        language.judge0_id
                    ),
                )),
                Some(entry) if !entry.name.contains(language.judge0_name_contains) => {
                    availability.unavailable.push((
                        language.slug,
                        format!(
                            "judge0 language id {} is \"{}\", expected a name containing \"{}\"",
                            language.judge0_id, entry.name, language.judge0_name_contains
                        ),
                    ))
                }
                Some(_) => availability.available.push(language.slug),
            }
        }

        Ok(availability)
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test judge_client`
Expected: PASS, 15 tests.

- [ ] **Step 5: Wire validation into boot**

In `src/main.rs`, after the migrations block and before `let db = Arc::new(db_connection);`, add:

```rust
    match slinttech_server::services::practice::judge::Judge0Client::new(&config) {
        Ok(judge) => match judge.validate_languages().await {
            Ok(availability) => {
                tracing::info!(
                    "Judge0 reachable: {} of {} languages available",
                    availability.available.len(),
                    availability.available.len() + availability.unavailable.len()
                );
                for (slug, reason) in &availability.unavailable {
                    tracing::warn!("Language '{}' is unavailable: {}", slug, reason);
                }
            }
            // The judge being down must not stop the server: the rest of the
            // platform does not depend on it, and submissions queue until it
            // returns.
            Err(error) => tracing::error!("Judge0 language validation failed: {}", error),
        },
        Err(error) => tracing::error!("Could not build the Judge0 client: {}", error),
    }
```

- [ ] **Step 6: Verify against the real judge**

Run: `cargo run` with the Task 1 stack up.
Expected: a log line reading `Judge0 reachable: 14 of 14 languages available`. Any `Language '<slug>' is unavailable` warning means the id in Task 3 is wrong for this Judge0 version — fix the table and re-run until all 14 are available.

- [ ] **Step 7: Format, lint and commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add src/services/practice/judge.rs src/main.rs tests/judge_client.rs
git commit -m "feat: validate judge0 language ids at boot"
```

---

## Task 9: Verify all 14 starter templates against the live judge

**Files:**
- Create: `tests/judge_live.rs`

**Interfaces:**
- Consumes: `Judge0Client::run_batch` (Task 7), `LANGUAGES` (Task 3), `verdict::aggregate` (Task 5).
- Produces: nothing consumed by later tasks. Its output is confidence that each configured language actually compiles and runs.

These tests are `#[ignore]`d so `cargo test` stays green without a judge. Run them with `cargo test --test judge_live -- --ignored`.

- [ ] **Step 1: Write the failing test**

Create `tests/judge_live.rs`:

```rust
//! Tests that require a running Judge0. All are #[ignore]d so the default
//! `cargo test` needs no judge; run them with:
//!
//!     cargo test --test judge_live -- --ignored --test-threads=2
//!
//! Set JUDGE0_URL and JUDGE0_AUTH_TOKEN first, or rely on the defaults in
//! deploy/judge0.

use slinttech_server::config::Config;
use slinttech_server::services::practice::judge::{
    ExecutionLimits, Judge0Client, TestCaseInput,
};
use slinttech_server::services::practice::languages;
use slinttech_server::services::practice::verdict::{aggregate, CaseResult, Verdict};

fn live_client() -> Judge0Client {
    // Config::from_env panics without DATABASE_URL and JWT_SECRET even though
    // neither is used here, so these tests need the project's .env present.
    let config = Config::from_env();
    Judge0Client::new(&config).expect("judge client builds")
}

fn sum_case() -> Vec<TestCaseInput> {
    vec![TestCaseInput {
        ordinal: 1,
        stdin: "17 25\n".to_string(),
        expected_output: "42\n".to_string(),
    }]
}

/// Runs `source` in `slug` against the 17+25 case and returns the verdict.
async fn verdict_for(slug: &str, source: &str) -> Verdict {
    let language = languages::by_slug(slug).expect("language is configured");
    let limits = ExecutionLimits::default();

    let results = live_client()
        .run_batch(language, source, &sum_case(), limits)
        .await
        .unwrap_or_else(|error| panic!("{slug}: judge call failed: {error}"));

    let outcome = aggregate(&results, limits.memory_limit_kb);

    if outcome.verdict != Verdict::Accepted {
        panic!(
            "{slug}: expected Accepted, got {:?}\ncompile_output: {:?}\nstderr: {:?}\nstdout: {:?}",
            outcome.verdict,
            outcome.compile_output,
            outcome.error_message,
            results[0].stdout,
        );
    }

    outcome.verdict
}

/// Confirms the shipped starter template at least compiles and runs. It reads
/// stdin and produces no output, so it is checked against an empty expectation.
async fn starter_template_runs(slug: &str) {
    let language = languages::by_slug(slug).expect("language is configured");
    let limits = ExecutionLimits::default();

    let cases = vec![TestCaseInput {
        ordinal: 1,
        stdin: "1 2\n".to_string(),
        expected_output: String::new(),
    }];

    let results = live_client()
        .run_batch(language, language.starter_code, &cases, limits)
        .await
        .unwrap_or_else(|error| panic!("{slug}: judge call failed: {error}"));

    let outcome = aggregate(&results, limits.memory_limit_kb);

    assert_ne!(
        outcome.verdict,
        Verdict::CompilationError,
        "{slug}: the shipped starter template does not compile:\n{}",
        outcome.compile_output.unwrap_or_default()
    );
    assert_ne!(
        outcome.verdict,
        Verdict::RuntimeError,
        "{slug}: the shipped starter template crashes:\n{}",
        outcome.error_message.unwrap_or_default()
    );
}

/// Generates one `#[ignore]`d test per language, so a broken language is named
/// in the test output rather than hidden inside a loop.
macro_rules! language_tests {
    ($( $test_name:ident => $slug:literal, $source:expr ; )*) => {
        $(
            #[tokio::test]
            #[ignore = "requires a running Judge0"]
            async fn $test_name() {
                starter_template_runs($slug).await;
                verdict_for($slug, $source).await;
            }
        )*
    };
}

language_tests! {
    python_runs => "python", r#"import sys

data = sys.stdin.read().split()
print(int(data[0]) + int(data[1]))
"#;

    javascript_runs => "javascript", r#"const data = require("fs")
  .readFileSync(0, "utf8")
  .split(/\s+/)
  .filter((token) => token.length > 0);

console.log(Number(data[0]) + Number(data[1]));
"#;

    typescript_runs => "typescript", r#"declare function require(name: string): any;

const data: string[] = require("fs")
  .readFileSync(0, "utf8")
  .split(/\s+/)
  .filter((token: string) => token.length > 0);

console.log(Number(data[0]) + Number(data[1]));
"#;

    ruby_runs => "ruby", r#"a, b = $stdin.read.split.map(&:to_i)
puts a + b
"#;

    php_runs => "php", r#"<?php

$data = preg_split('/\s+/', trim(stream_get_contents(STDIN)), -1, PREG_SPLIT_NO_EMPTY);
echo ((int) $data[0] + (int) $data[1]) . "\n";
"#;

    java_runs => "java", r#"import java.util.Scanner;

public class Main {
    public static void main(String[] args) {
        Scanner scanner = new Scanner(System.in);
        System.out.println(scanner.nextInt() + scanner.nextInt());
    }
}
"#;

    kotlin_runs => "kotlin", r#"fun main() {
    val values = generateSequence(::readLine)
        .flatMap { line -> line.trim().split(Regex("\\s+")).asSequence() }
        .filter { token -> token.isNotEmpty() }
        .map { token -> token.toLong() }
        .toList()

    println(values[0] + values[1])
}
"#;

    csharp_runs => "csharp", r#"using System;

public class Program
{
    public static void Main()
    {
        string[] data = Console.In.ReadToEnd()
            .Split(new[] { ' ', '\t', '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        Console.WriteLine(long.Parse(data[0]) + long.Parse(data[1]));
    }
}
"#;

    swift_runs => "swift", r#"import Foundation

var input = ""
while let line = readLine(strippingNewline: false) {
    input += line
}

let data = input
    .split(whereSeparator: { $0 == " " || $0 == "\n" || $0 == "\t" || $0 == "\r" })
    .map(String.init)

print(Int(data[0])! + Int(data[1])!)
"#;

    dart_runs => "dart", r#"import 'dart:io';

void main() {
  final buffer = StringBuffer();
  String? line;
  while ((line = stdin.readLineSync()) != null) {
    buffer.writeln(line);
  }

  final data = buffer
      .toString()
      .split(RegExp(r'\s+'))
      .where((token) => token.isNotEmpty)
      .toList();

  print(int.parse(data[0]) + int.parse(data[1]));
}
"#;

    c_runs => "c", r#"#include <stdio.h>

int main(void) {
    long a, b;
    if (scanf("%ld %ld", &a, &b) != 2) {
        return 1;
    }
    printf("%ld\n", a + b);
    return 0;
}
"#;

    cpp_runs => "cpp", r#"#include <bits/stdc++.h>

using namespace std;

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    long long a, b;
    cin >> a >> b;
    cout << a + b << "\n";
    return 0;
}
"#;

    go_runs => "go", r#"package main

import (
	"bufio"
	"fmt"
	"os"
)

func main() {
	reader := bufio.NewReader(os.Stdin)
	var a, b int64
	if _, err := fmt.Fscan(reader, &a, &b); err != nil {
		return
	}
	fmt.Println(a + b)
}
"#;

    rust_runs => "rust", r#"use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    let mut parts = input.split_whitespace();
    let a: i64 = parts.next().expect("first value").parse().expect("an integer");
    let b: i64 = parts.next().expect("second value").parse().expect("an integer");

    println!("{}", a + b);
}
"#;
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn cpp17_features_compile() {
    // Proves the -std=c++17 compiler option actually reached the compiler:
    // structured bindings are a C++17 feature and fail under gnu++14.
    verdict_for(
        "cpp",
        r#"#include <bits/stdc++.h>

using namespace std;

int main() {
    pair<long long, long long> values;
    cin >> values.first >> values.second;
    auto [a, b] = values;
    cout << a + b << "\n";
    return 0;
}
"#,
    )
    .await;
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --test judge_live -- --ignored --test-threads=2`
Expected: 15 tests pass. Compiled languages take several seconds each, so the whole run takes a few minutes.

- [ ] **Step 3: Fix whatever fails**

A failure here is real information, not a flaky test. Each panic message names the language and prints the compiler or runtime output. Two likely causes:

- **Compilation error in the starter template** — fix `starter_code` in `src/services/practice/languages.rs` and re-run.
- **Wrong `judge0_id`** — the language is compiling as something else entirely. Re-check against Task 1 Step 8.

If a language cannot be made to work, remove it from `LANGUAGES` and say so in the commit message. Shipping a language that does not run is worse than shipping thirteen.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add tests/judge_live.rs src/services/practice/languages.rs
git commit -m "test: verify all 14 language templates against a live judge"
```

---

## Task 10: Adversarial sandbox suite

**Files:**
- Modify: `tests/judge_live.rs`

**Interfaces:**
- Consumes: `Judge0Client::run_batch` (Task 7), `verdict::aggregate` (Task 5).
- Produces: nothing. This is the gate that decides whether Phase 1 is finished.

- [ ] **Step 1: Write the failing test**

Append to `tests/judge_live.rs`:

```rust
use slinttech_server::services::practice::verdict::JudgeOutcome;

/// Runs hostile source and returns the outcome. Unlike `verdict_for` this does
/// not require Accepted — the point is that the judge survives and reports
/// something bounded.
async fn hostile_outcome(slug: &str, source: &str) -> (JudgeOutcome, Vec<CaseResult>) {
    let language = languages::by_slug(slug).expect("language is configured");
    let limits = ExecutionLimits::default();

    let cases = vec![TestCaseInput {
        ordinal: 1,
        stdin: String::new(),
        expected_output: "never\n".to_string(),
    }];

    let results = live_client()
        .run_batch(language, source, &cases, limits)
        .await
        .unwrap_or_else(|error| panic!("{slug}: the judge did not return: {error}"));

    // The raw cases come back too: several checks below inspect what the
    // program printed, and `JudgeOutcome` deliberately does not carry stdout.
    let outcome = aggregate(&results, limits.memory_limit_kb);
    (outcome, results)
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn an_infinite_loop_is_killed_at_the_time_limit() {
    let (outcome, _) = hostile_outcome("python", "while True:\n    pass\n").await;
    assert_eq!(outcome.verdict, Verdict::TimeLimitExceeded);
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn a_sleeping_program_is_killed_by_the_wall_clock_limit() {
    // Burns no CPU, so only the wall-time limit stops it.
    let (outcome, _) = hostile_outcome("python", "import time\ntime.sleep(30)\n").await;
    assert_eq!(outcome.verdict, Verdict::TimeLimitExceeded);
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn a_fork_bomb_is_contained() {
    let (outcome, _) = hostile_outcome(
        "python",
        "import os\nwhile True:\n    os.fork()\n",
    )
    .await;

    assert_ne!(outcome.verdict, Verdict::Accepted);
    // The specific verdict depends on whether the process cap or the time limit
    // trips first; either is a contained failure.
    assert!(
        matches!(
            outcome.verdict,
            Verdict::RuntimeError | Verdict::TimeLimitExceeded | Verdict::InternalError
        ),
        "got {:?}",
        outcome.verdict
    );
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn a_huge_allocation_is_refused() {
    let (outcome, _) = hostile_outcome("python", "data = bytearray(10 * 1024 * 1024 * 1024)\n").await;

    assert!(
        matches!(
            outcome.verdict,
            Verdict::MemoryLimitExceeded | Verdict::RuntimeError
        ),
        "got {:?}",
        outcome.verdict
    );
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn outbound_network_access_fails() {
    // The single most important sandbox property. If this test fails,
    // submitted code can exfiltrate whatever it can reach.
    let (outcome, results) = hostile_outcome(
        "python",
        r#"import socket

try:
    socket.create_connection(("1.1.1.1", 80), timeout=5)
    print("NETWORK REACHABLE")
except Exception as error:
    print("blocked:", type(error).__name__)
"#,
    )
    .await;

    // The program reports its own result on stdout, so that is what must be
    // checked. Asserting only on the verdict would pass trivially, because the
    // expected output is "never" and can therefore never match.
    let stdout = results[0].stdout.clone().unwrap_or_default();
    assert!(
        !stdout.contains("NETWORK REACHABLE"),
        "the sandbox reached the network: {stdout}"
    );
    assert!(
        stdout.contains("blocked"),
        "expected the connection to be refused, got: {stdout}"
    );
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn the_sandbox_cannot_read_host_secrets() {
    let (_outcome, results) = hostile_outcome(
        "python",
        r#"paths = ["/etc/shadow", "/root/.ssh/id_rsa", "/proc/1/environ"]
for path in paths:
    try:
        with open(path, "rb") as handle:
            print("READABLE", path, len(handle.read()))
    except Exception as error:
        print("blocked", path, type(error).__name__)
"#,
    )
    .await;

    // Whatever the verdict, no path may report as READABLE.
    let stdout = results[0].stdout.clone().unwrap_or_default();
    assert!(
        !stdout.contains("READABLE"),
        "the sandbox exposed a host path: {stdout}"
    );
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn writing_an_oversized_file_is_stopped() {
    let (outcome, results) = hostile_outcome(
        "python",
        r#"with open("big.bin", "wb") as handle:
    for _ in range(1024):
        handle.write(b"x" * 1024 * 1024)
print("WROTE 1GB")
"#,
    )
    .await;

    assert_ne!(outcome.verdict, Verdict::Accepted);
    let stdout = results[0].stdout.clone().unwrap_or_default();
    assert!(
        !stdout.contains("WROTE 1GB"),
        "the file-size limit did not stop the write: {stdout}"
    );
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn unbounded_recursion_is_a_runtime_error() {
    let (outcome, _) = hostile_outcome(
        "c",
        r#"#include <stdio.h>

long long depth = 0;

void recurse(void) {
    depth++;
    recurse();
}

int main(void) {
    recurse();
    printf("%lld\n", depth);
    return 0;
}
"#,
    )
    .await;

    assert_eq!(outcome.verdict, Verdict::RuntimeError);
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn a_non_zero_exit_is_a_runtime_error() {
    let (outcome, _) = hostile_outcome("python", "import sys\nsys.exit(3)\n").await;
    assert_eq!(outcome.verdict, Verdict::RuntimeError);
}

#[tokio::test]
#[ignore = "requires a running Judge0"]
async fn the_judge_still_works_after_the_adversarial_suite() {
    // Run this last. A judge that accepts hostile code but is wedged afterwards
    // has still failed.
    verdict_for("python", "import sys\n\ndata = sys.stdin.read().split()\nprint(int(data[0]) + int(data[1]))\n").await;
}
```

- [ ] **Step 2: Run the suite**

Run: `cargo test --test judge_live -- --ignored --test-threads=1`

Single-threaded on purpose: several of these deliberately exhaust judge resources, and running them concurrently makes failures ambiguous.

Expected: 10 adversarial tests pass, plus the 15 from Task 9.

- [ ] **Step 3: Confirm the host survived**

Run:

```bash
cd deploy/judge0 && docker compose ps
```

Expected: all four services still `running`, none restarting. Then check the host itself is healthy:

```bash
uptime && free -h && df -h /
```

Expected: load returning to normal, memory not exhausted, disk not filled by the 1GB-file test.

**If any adversarial test hangs the host, Phase 1 is not complete.** Re-check `judge0.conf` ceilings and the `deploy.resources.limits` on the workers service, and do not proceed to Phase 2.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add tests/judge_live.rs
git commit -m "test: add adversarial sandbox suite for the judge"
```

---

## Phase 1 exit criteria

Phase 1 is complete when all of the following are true:

- [ ] `cargo test` passes with no judge running (the `#[ignore]`d suites are skipped).
- [ ] `cargo test --test judge_live -- --ignored --test-threads=1` passes with the judge running: 25 tests.
- [ ] `cargo run` logs `Judge0 reachable: 14 of 14 languages available`.
- [ ] `curl http://127.0.0.1:2358/about` without the auth header returns 401.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] The judge stack is running on the deployment host, bound to loopback, with generated secrets in `judge0.conf.local`.

No user-facing surface exists yet, and nothing in the running application depends on the judge. That is deliberate: Phase 1 can be verified, and shipped, in isolation.

## What Phase 2 picks up

Phase 2 (catalog and schema) creates the seven migrations and entities from spec §6, the `catalog.rs` read side, and the admin authoring routes. It consumes `languages::LANGUAGES` for `GET /api/practice/languages` and nothing else from this phase. Phase 3 (submission pipeline) is the first consumer of `Judge0Client::run_batch` and `verdict::aggregate`.
