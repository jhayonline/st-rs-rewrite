# Coding Practice Arena — Design

Date: 2026-08-20
Status: Approved design, pending implementation plan
Repos: `slinttech-server` (backend, this repo), `slint-tech-dashboard` (frontend)

## 1. Summary

A LeetCode-style coding practice arena added to SlintTech. Users browse and
search a catalog of programming problems by difficulty and topic, open a
problem in a browser-based Monaco editor, pick one of 14 languages, run their
code against sample or custom input, and submit it to a sandboxed judge. The
judge compiles and executes the submission against hidden test cases and
returns a verdict — Accepted, Wrong Answer, Compilation Error, Runtime Error,
Time Limit Exceeded, Memory Limit Exceeded — with runtime, memory usage and
score. A first Accepted submission awards the problem's points once, feeding a
global leaderboard that updates live over WebSockets.

## 2. Goals

- Browse, search and filter problems by difficulty, topic and personal solve status.
- Solve in-browser with Monaco across 14 languages.
- Run against sample or custom input without recording a submission.
- Submit to a secure sandboxed judge and receive a verdict with runtime, memory and score.
- Award points on first Accept and maintain a global ranking.
- Push verdict and leaderboard changes to connected clients in real time.

## 2.1 Access policy

The arena is open to **every authenticated user** — mentee, mentor or admin —
regardless of course enrolment, payment state or approval state. It is
deliberately not behind the existing PaymentWall or PendingApproval gates.
Authoring routes are the only ones requiring a role.

Non-admin catalog routes return only problems with `status = 'published'`.
`draft` and `archived` problems are invisible outside the admin routes, and a
submission to a non-published problem is rejected.

## 3. Non-goals

- LeetCode-style function-signature judging. This system is stdin/stdout.
- Contests, timed rounds, virtual participation.
- Mentor- or course-assigned problem sets. The arena is standalone in this phase.
- Discussion threads, editorials, community solutions.
- Plagiarism detection.
- A good phone editing experience.

## 4. Locked decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | **Self-hosted Judge0** as the execution engine | Inherits `isolate` cgroup/namespace sandboxing, 60+ language toolchains, and per-run time/memory metrics. Writing an equivalent sandbox is a project in itself and the failure mode is remote code execution on the host. |
| 2 | **Standalone arena tables**, existing `tasks`/`task_submissions` untouched | Those model human-reviewed deliverables owned by a mentor. Overloading them would force every existing query to learn a new shape and would put a working flow at risk. A course/problem link can be added later additively. |
| 3 | **First-accept, fixed points by difficulty** | Cheap to maintain incrementally, cannot be farmed by resubmitting, and a user's score never changes because of someone else's activity. |
| 4 | **stdin/stdout I/O contract** | Maps 1:1 onto Judge0 with no glue. Adding a language is a config entry. Authoring a problem needs only text pairs, no per-language driver code. |
| 5 | **Postgres as the work queue** (`SELECT … FOR UPDATE SKIP LOCKED`) | Submissions survive process restarts, in-flight judging is capped for free, retry is a column. No Redis, no separate worker binary. |
| 6 | **Rank computed at query time**, languages in a **Rust config module** | A stored rank column means every solve rewrites many rows and can drift from the scores it summarises. A language is never a pure data change — it also needs a Monaco mode and a starter template shipped with the frontend. |

## 5. Architecture

```
Browser (React + Monaco)
   |  REST  /api/practice/*        (JWT bearer)
   |  WS    /api/ws                (first-message auth)
   v
Salvo server  ──────────────────────────────────────────────
   controllers/practice.rs
   services/practice/{catalog,submission,judge,worker,scoring,languages}
   realtime/{hub,ws}
   |                     ^
   | app Postgres        | broadcast
   v                     |
 problems, test cases, submissions, solves, rankings
   |
   | worker claims queued rows, HTTP to judge (private network only)
   v
Judge0 stack (separate docker compose: api, workers, own postgres, own redis)
   isolate sandbox, no network, cpu/mem/pid/file limits
```

The judge stack holds no application secrets and has no route to the
application database. Its compromise yields a machine that runs untrusted code,
which is its purpose.

## 6. Data model

Seven new tables, seven migrations, all additive. Conventions match the existing
schema: `uuid` primary keys defaulting to `gen_random_uuid()`,
`timestamp_with_time_zone`, enum-like values as `text` with a default,
`created_at`/`updated_at` maintained by an `ActiveModelBehavior::before_save`
hook.

### 6.1 `problems`

| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | |
| `slug` | text UNIQUE NOT NULL | drives `/practice/two-sum` |
| `title` | text NOT NULL | |
| `difficulty` | text NOT NULL | `easy` \| `medium` \| `hard` |
| `statement_md` | text NOT NULL | markdown |
| `constraints_md` | text NULL | markdown |
| `points` | integer NOT NULL | defaults from difficulty at creation; stored per problem |
| `time_limit_ms` | integer NOT NULL DEFAULT 2000 | CPU limit per test case |
| `memory_limit_kb` | integer NOT NULL DEFAULT 262144 | 256 MB |
| `status` | text NOT NULL DEFAULT `'draft'` | `draft` \| `published` \| `archived` |
| `author_id` | uuid NULL FK → `user_profiles.id` | |
| `starter_code` | jsonb NULL | `{language_slug: source}`; null means use the language default |
| `total_submissions` | integer NOT NULL DEFAULT 0 | denormalised for the browse list |
| `total_accepted` | integer NOT NULL DEFAULT 0 | denormalised for the browse list |
| `created_at`, `updated_at` | timestamptz | |

Indexes: unique on `slug`; `(status, difficulty)`; GIN trigram on `title` and
`slug`, matching the columns §11's search actually queries.

Default points by difficulty: easy 10, medium 25, hard 50.

### 6.2 `topics` and `problem_topics`

`topics`: `id` uuid PK, `slug` text unique, `name` text, `created_at`.

`problem_topics`: `problem_id` uuid, `topic_id` uuid, composite primary key,
both foreign keys `ON DELETE CASCADE`. Index on `topic_id` for the reverse
lookup.

A join table rather than a JSON array column, because filtering by topic is a
first-class requirement and this makes it an indexed equality test. It also
yields the sidebar's per-topic counts directly.

### 6.3 `problem_test_cases`

| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | |
| `problem_id` | uuid NOT NULL FK ON DELETE CASCADE | |
| `input` | text NOT NULL | fed to stdin |
| `expected_output` | text NOT NULL | compared after trailing-whitespace trim |
| `is_sample` | boolean NOT NULL DEFAULT false | samples are public |
| `ordinal` | integer NOT NULL | evaluation and display order |
| `explanation` | text NULL | rendered for samples only |
| `created_at`, `updated_at` | timestamptz | |

Index: `(problem_id, ordinal)`.

**Hidden test cases must never reach a client.** The entity is never serialised
directly into a response. Only purpose-built response models are, and the model
used by the public problem-detail route filters on `is_sample = true`. The single
exception is `GET /api/practice/admin/problems/{id}`, behind `RequireRole`
admin. This rule gets a dedicated regression test.

Output comparison trims trailing whitespace on each line and trailing newlines
at end of output, matching Judge0's default expected-output behaviour. Anything
stricter generates false Wrong Answers from invisible characters.

### 6.4 `problem_submissions`

Both the durable record and the work queue.

| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | |
| `problem_id` | uuid NOT NULL FK | |
| `user_id` | uuid NOT NULL FK → `user_profiles.id` | |
| `language` | text NOT NULL | our slug |
| `judge0_language_id` | integer NOT NULL | snapshotted so reconfiguring a language never rewrites history |
| `source_code` | text NOT NULL | ≤ 64 KB, enforced before insert |
| `status` | text NOT NULL DEFAULT `'queued'` | `queued` \| `running` \| `completed` \| `failed` |
| `verdict` | text NULL | see §9 |
| `passed_tests` | integer NOT NULL DEFAULT 0 | |
| `total_tests` | integer NOT NULL DEFAULT 0 | |
| `runtime_ms` | integer NULL | max across test cases |
| `memory_kb` | integer NULL | max across test cases |
| `score_awarded` | integer NOT NULL DEFAULT 0 | |
| `compile_output` | text NULL | truncated to 8 KB |
| `error_message` | text NULL | stderr, truncated to 8 KB |
| `failed_test_ordinal` | integer NULL | index only; hidden case content is never exposed |
| `attempts` | integer NOT NULL DEFAULT 0 | worker retry counter |
| `locked_at` | timestamptz NULL | worker claim time, drives the stale reaper |
| `judged_at` | timestamptz NULL | |
| `created_at`, `updated_at` | timestamptz | |

Indexes: `(user_id, problem_id, created_at DESC)`; `(problem_id)`; partial index
on `(created_at)` where `status = 'queued'`; partial index on `(locked_at)`
where `status = 'running'`.

### 6.5 `user_problem_solves`

The first-accept ledger.

| Column | Type |
|---|---|
| `user_id` | uuid NOT NULL FK |
| `problem_id` | uuid NOT NULL FK |
| `submission_id` | uuid NOT NULL FK |
| `points_awarded` | integer NOT NULL |
| `solved_at` | timestamptz NOT NULL |

**PRIMARY KEY (`user_id`, `problem_id`).**

That composite key is the award-once guarantee. The worker performs
`INSERT … ON CONFLICT (user_id, problem_id) DO NOTHING` and increments the
user's score only when a row was actually inserted. The deduplication is a
database constraint rather than a read-then-write in application code, so two
submissions of the same solution completing concurrently cannot double-award.
An `if already_solved { skip }` check in Rust has a race window between the read
and the write; this has none.

### 6.6 `user_rankings`

| Column | Type | Notes |
|---|---|---|
| `user_id` | uuid PK FK → `user_profiles.id` | |
| `total_score` | integer NOT NULL DEFAULT 0 | |
| `problems_solved` | integer NOT NULL DEFAULT 0 | |
| `easy_solved`, `medium_solved`, `hard_solved` | integer NOT NULL DEFAULT 0 | |
| `last_solved_at` | timestamptz NULL | rank tie-breaker |
| `updated_at` | timestamptz | |

Index: `(total_score DESC, last_solved_at ASC)`.

Rank is **not** a column. It is computed per query:

```sql
RANK() OVER (ORDER BY total_score DESC, last_solved_at ASC NULLS LAST)
```

Ties share a rank, broken by whoever reached the score first. A stored rank
column would require rewriting up to N rows on every solve, and each of those
writes is an opportunity to drift out of sync with `total_score`.

## 7. Language configuration

`services/practice/languages.rs` holds a static table. Each entry: slug,
display name, Judge0 language id, Monaco mode id, file extension, default
stdin/stdout starter template.

| Slug | Language | Kind | Notes |
|---|---|---|---|
| `python` | Python 3 | interpreted | |
| `javascript` | JavaScript (Node) | interpreted | |
| `typescript` | TypeScript (Node) | compiled → JS | `tsc` then node; verify template compiles without `@types/node` |
| `ruby` | Ruby | interpreted | |
| `php` | PHP | interpreted | |
| `java` | Java | compiled | public class **must** be `Main` |
| `kotlin` | Kotlin | compiled | file is `Main.kt`, needs top-level `fun main()` |
| `csharp` | C# | compiled | Mono on Judge0 CE; any `static Main` |
| `swift` | Swift | compiled | |
| `dart` | Dart | compiled | |
| `c` | C | compiled | gcc |
| `cpp` | C++17 | compiled | g++ |
| `go` | Go | compiled | |
| `rust` | Rust | compiled | slowest compile of the set |

Monaco ships a built-in mode for all fourteen, so no custom grammars are needed.

**Judge0 language ids are never hardcoded from documentation.** They shift
between Judge0 releases, and a stale id silently judges submissions against the
wrong compiler. At boot the server calls Judge0's `GET /languages` and verifies
every configured id exists and its reported name matches expectation, logging
loudly and marking the language unavailable on mismatch.

Availability is a runtime set computed at boot and held in application state,
not a column. `GET /languages` returns only available languages, and a submit
naming an unavailable one is rejected with 400. A Judge0 upgrade that drops a
language therefore removes it from the picker instead of producing mysterious
verdicts.

**Compile time is separate from run time.** For the compiled languages the
`cpu_time_limit` applies to execution only; Judge0 tracks compilation separately
and reports failure as `compilation_error`. This matters most for Rust, where a
user must never see Time Limit Exceeded because `rustc` was slow.

**Entry-point guards.** Java submissions whose public class is not `Main`, and
Kotlin submissions with no top-level `main`, are rejected at submit time with a
plain-language message. Without the guard the user gets a compiler error about a
filename they never chose.

Every language's starter template must be verified against the live Judge0
instance — compiled and run — before it ships. Fourteen templates is real work
and is planned as such, with one integration test per language.

## 8. Services and modules

```
src/entities/            problems, topics, problem_topics, problem_test_cases,
                         problem_submissions, user_problem_solves, user_rankings
src/models/practice.rs   request and response structs
src/services/practice/
  mod.rs
  catalog.rs             browse, search, filter, problem detail
  submission.rs          enqueue, status, user history
  judge.rs               Judge0 HTTP client and verdict mapping
  worker.rs              claim loop and stale-claim reaper
  scoring.rs             award-once and ranking queries
  languages.rs           the 14-language config
src/realtime/
  hub.rs                 broadcast channel and event types
  ws.rs                  Salvo WebSocket handler
src/controllers/practice.rs
migration/src/m20260820_*_create_*.rs
```

A directory rather than the flat `services/<area>.rs` convention used elsewhere:
as one file this would exceed a thousand lines mixing an HTTP client, a worker
loop and scoring arithmetic. Each module has a single purpose and a stated
dependency set. `judge.rs` knows nothing about the domain — it takes source,
language id and a list of `(stdin, expected)` pairs and returns raw per-case
results, so it is testable without a database.

## 9. Submission lifecycle

1. **Enqueue.** `POST /api/practice/problems/{slug}/submit` with
   `{language, source_code}`. Validates: problem published; language known and
   available; source non-empty and ≤ 64 KB; entry-point guard for Java/Kotlin;
   rate limit. Inserts a row with `status = 'queued'` and returns
   `202 {submission_id, status}`. The handler does not contact Judge0.

2. **Claim.** A single Tokio task started at boot polls every 250 ms:

   ```sql
   UPDATE problem_submissions
      SET status = 'running', locked_at = now(), attempts = attempts + 1
    WHERE id IN (
      SELECT id FROM problem_submissions
       WHERE status = 'queued'
       ORDER BY created_at
       LIMIT $max_inflight
       FOR UPDATE SKIP LOCKED)
   RETURNING *;
   ```

3. **Dispatch.** Loads all test cases for the problem, posts a single Judge0
   batch with one entry per case (`source_code`, `language_id`, `stdin`,
   `expected_output`, `cpu_time_limit`, `wall_time_limit`, `memory_limit`,
   `max_processes_and_or_threads`, `max_file_size`, `enable_network: false`),
   then polls the batch tokens with backoff from 100 ms to 1 s until every case
   is terminal or an overall deadline expires.

4. **Aggregate.** See §10.

5. **Persist.** On `accepted`, in one transaction: insert into
   `user_problem_solves` with `ON CONFLICT DO NOTHING`; if and only if a row was
   inserted, upsert `user_rankings` (add points, increment `problems_solved` and
   the difficulty counter, set `last_solved_at`); write `score_awarded`; update
   the problem's `total_submissions` and `total_accepted`.

6. **Broadcast — after the transaction commits, never inside it.** A rollback
   that has already pushed a new score to every connected client leaves the
   leaderboard displaying points that do not exist.

### 9.1 Failure handling

- **Judge0 unreachable or 5xx** — the row returns to `queued` with backoff. Only
  after 3 attempts does it become `status = 'failed'`, `verdict =
  'internal_error'`. A judge outage delays submissions; it never loses them and
  never awards points.
- **Stale-claim reaper** — rows in `running` with `locked_at` older than 5
  minutes return to `queued`. This is what makes a mid-judge deploy or crash a
  non-event, and it is the concrete payoff for choosing a durable queue.
- **Partial batch results** — treated as not-yet-terminal until the deadline,
  then `internal_error`.

### 9.2 Run is a separate path

`POST /api/practice/problems/{slug}/run` executes against the sample cases or a
user-supplied stdin, creates no submission row, awards nothing, and calls Judge0
directly rather than queueing. It is interactive, users expect it to be
immediate, and a lost run costs one button press. It carries a tighter rate
limit and its own semaphore so a burst of runs cannot starve real submissions of
judge capacity.

## 10. Verdict mapping

Deterministic, first-failure-by-ordinal.

1. Any case with Judge0 status 6 → `compilation_error`, store `compile_output`.
   Compilation is a property of the submission, not of a test case, so it wins
   outright.
2. Otherwise the lowest-ordinal case that is not status 3 decides the verdict:

| Judge0 status | Verdict |
|---|---|
| 3 Accepted | (continue) |
| 4 Wrong Answer | `wrong_answer` |
| 5 Time Limit Exceeded | `time_limit_exceeded` |
| 7–12 (SIGSEGV, SIGXFSZ, SIGFPE, SIGABRT, NZEC, other) | `runtime_error` |
| 13, 14 (internal error, exec format error) | `internal_error` |

3. All cases status 3 → `accepted`.
4. `runtime_ms` and `memory_kb` are the maxima across cases.
5. **Memory limit is inferred, not reported.** Judge0 surfaces an OOM kill as a
   `SIGSEGV` runtime error. When a case fails with status 7 and its reported
   memory is at or near `memory_limit_kb`, the verdict is reclassified as
   `memory_limit_exceeded`.

`failed_test_ordinal` records which case failed. For hidden cases only the index
is exposed — never the input, expected output, or actual output.

## 11. API surface

All routes are under `/api/practice` and behind `AuthMiddleware`.

### Read

| Route | Returns |
|---|---|
| `GET /languages` | the 14 entries: slug, name, monaco mode, extension, default source |
| `GET /topics` | topics with problem counts |
| `GET /problems` | paged list; filters `search`, `difficulty`, `topic`, `status=solved\|attempted\|unsolved`, `sort`, `page`, `per_page` |
| `GET /problems/{slug}` | statement, constraints, limits, per-language starter code, **sample cases only**, caller's solve status |
| `GET /submissions/{id}` | full verdict; 403 unless owner or admin |
| `GET /submissions?problem=&page=` | caller's submission history |
| `GET /leaderboard?page=&per_page=` | ranked page plus a `me` block, so the caller's own row is always present |
| `GET /me` | caller's score, solved counts, rank |

### Write

| Route | Behaviour |
|---|---|
| `POST /problems/{slug}/run` | executes against samples or custom stdin, returns per-case results inline, no row, no score |
| `POST /problems/{slug}/submit` | `202 {submission_id, status: "queued"}` |

### Authoring — `RequireRole` admin

| Route | Behaviour |
|---|---|
| `POST /problems` | create |
| `PUT /problems/{id}` | update |
| `DELETE /problems/{id}` | soft delete to `archived` |
| `POST /problems/{id}/test-cases`, `PUT /test-cases/{id}`, `DELETE /test-cases/{id}` | test-case CRUD |
| `GET /admin/problems/{id}` | the only route that returns hidden test cases |

Search is `ILIKE` on title and slug backed by a `pg_trgm` index — not
full-text search. At the scale this catalog will reach, trigram matching is the
proportionate tool.

## 12. WebSocket contract

Endpoint `GET /api/ws`.

**Authentication is by first message, not query string.** The client connects,
sends `{type: "auth", token}` within 5 seconds, and receives `{type: "auth_ok",
userId}` or is closed with 4401. Browsers cannot set an `Authorization` header
on a WebSocket handshake, and the obvious alternative — `?token=<jwt>` — writes
a live credential into access logs, proxy logs and browser history, where these
long-lived JWTs would remain valid.

Server events:

| Event | Delivery | Payload |
|---|---|---|
| `submission_update` | only to the submitting user | `submissionId`, `status`, `verdict`, `passedTests`, `totalTests`, `runtimeMs`, `memoryKb`, `scoreAwarded` |
| `leaderboard_update` | broadcast | top-N snapshot plus the recipient's own row |
| `resync` | to a lagging connection | instruction to refetch over REST |

Payloads are emitted through the same `camelizeKeys` helper `lib/api.ts` already
uses, so the wire format stays consistent with REST rather than growing a second
naming convention.

Implementation: a `tokio::sync::broadcast` channel in application state; one
task per connection filters user-targeted events by `user_id`. Keepalive
ping/pong every 30 seconds.

Three robustness rules:

1. **Leaderboard broadcasts are coalesced to at most one per second.** Without
   it, a burst of solves during a class session repeatedly fans a full top-50
   payload out to every socket.
2. **A lagging client is told to resync, not replayed.** The channel is bounded;
   a slow connection receives `Lagged` and is sent `resync`. Buffering missed
   events for slow sockets turns one slow client into unbounded memory growth.
3. **The WebSocket is strictly an accelerator.** Every event has a REST
   equivalent, the client polls when the socket is down, and no data is
   obtainable only over the socket. If this layer fails in production the
   feature degrades to slightly less live, not broken.

## 13. Judge0 deployment and security posture

### Topology

Judge0 runs as its own `docker compose` stack — API, workers, **its own
Postgres, its own Redis** — bound to `127.0.0.1:2358` or a private Docker
network. It is never publicly exposed. The Salvo server is its only client.

Separate datastores are a blast-radius decision, not tidiness. Judge0 ships
**no authentication by default**: anyone who can reach that port can execute
arbitrary code on the host. Sharing the application Postgres would put
`user_profiles`, contract files and payment records one query away from a single
exposed port. As designed the judge stack holds no `JWT_SECRET`, no
`PAYSTACK_SECRET_KEY`, no `CLOUDINARY_API_SECRET`, and no application
`DATABASE_URL`.

Additional hardening: Judge0's `AUTHN_HEADER` token as a second layer, and CPU
and memory quotas on the worker services so a fork bomb starves the judge rather
than the API.

### Host requirements — verify before writing code

- Workers require `privileged` mode; `isolate` uses cgroups and namespaces
  directly. A shared PaaS cannot run this. A VPS under your control can.
- **cgroup v1.** Judge0 1.13.x requires it, and current distributions boot
  cgroup v2 by default, needing `systemd.unified_cgroup_hierarchy=0` on the
  kernel command line. Newer Judge0 releases support v2. This is the most common
  Judge0 startup failure and it is a host kernel flag, so it is an early task in
  the plan rather than a discovery during integration.

### Per-execution limits

| Setting | Value | Purpose |
|---|---|---|
| `enable_network` | **false** | The most important flag. Also pinned globally via `ALLOW_ENABLE_NETWORK=false` so application code cannot re-enable it. |
| `cpu_time_limit` | problem's `time_limit_ms`, default 2 s | |
| `cpu_extra_time` | 0.5 s | |
| `wall_time_limit` | 2× CPU limit | catches `sleep` and blocking I/O, which burn no CPU |
| `memory_limit` | problem's `memory_limit_kb`, default 256 MB | |
| `max_processes_and_or_threads` | 60 | fork-bomb guard. **Not 1** — the JVM and Go runtime legitimately spawn threads, and too low a value fails valid submissions. |
| `max_file_size` | 1 MB | prevents filling the disk |
| `number_of_runs` | 1 | |
| `redirect_stderr_to_stdout` | false | stderr is needed separately for runtime-error messages |

`judge0.conf` sets matching **hard ceilings** (`MAX_CPU_TIME_LIMIT`,
`MAX_MEMORY_LIMIT`, `MAX_PROCESSES_AND_OR_THREADS`, `MAX_SUBMISSION_BATCH_SIZE`)
so a bug in request construction cannot ask for 60 seconds and receive it.
Limits enforced only by the caller are not limits.

### Application-side guards

Source ≤ 64 KB. Stored `stdout`, `stderr` and `compile_output` truncated to 8 KB
each. Submit rate-limited to 1 per 5 s and 60 per hour per user; run to 1 per
2 s. A semaphore bounds concurrent runs. Source code is never written to logs.

Rate limiting and the run semaphore are **in-process**, so their limits are per
server instance rather than global. That is correct for the current
single-instance deployment; running more than one instance would need a shared
counter. The `SKIP LOCKED` queue is unaffected — it is already correct across
instances.

### Adversarial test suite — a deliverable, not a hope

Each of the following must return a bounded verdict within its limit and leave
the host healthy afterwards: fork bomb; `while (true)`; 10 GB allocation;
outbound socket connect; reading `/etc/passwd`; writing a 1 GB file; unbounded
recursion; explicit process exit with a non-zero code. If any hangs the host,
the feature is not ready to ship.

### Residual risk

A determined kernel-level container escape is not defended against. Mitigation
is a patched host, no co-located secrets, and preferably the judge on its own
VM. For a practice arena this is a proportionate posture, but the risk is real
rather than zero.

## 14. Frontend

React 18 + Vite + Tailwind v4 + react-router-dom v7, in `slint-tech-dashboard`.

Reused: `lib/api.ts` for every request (inheriting bearer-token attachment and
case conversion), `useDebounce` for search, `ProtectedRoute` for route guarding,
`Toast` and `SkeletonLoader` for feedback and loading states. No new state
library.

### New dependencies

- `@monaco-editor/react` — **bundled locally, not loaded from its default
  jsDelivr CDN.** The same reasoning that ruled out a hosted judge: no third
  party in the critical path of a core feature. It also works on locked-down
  networks and survives a CDN outage.
- `react-markdown` + `remark-gfm` — problem statements, rendered sanitized.
  Authors are admins, but a stored-XSS path through an authoring UI is a classic.

**Monaco is roughly 5 MB and must be code-split** behind `React.lazy`, so
`/practice`, the leaderboard, and every unrelated page — including the marketing
homepage — never download it.

### Routes

| Route | Page |
|---|---|
| `/practice` | browse: debounced search, difficulty pills, topic sidebar with counts, solved/attempted/unsolved filter, paged list with solve marks |
| `/practice/:slug` | workspace: split pane, statement/samples/my-submissions tabs left, language selector + Monaco + Run/Submit + results panel right. `Ctrl+Enter` runs, `Ctrl+Shift+Enter` submits |
| `/practice/leaderboard` | ranked table with the caller's row pinned, live-updating |
| `/admin/practice/problems` | authoring: problem CRUD and the test-case editor with a sample/hidden toggle |

### Components and hooks

Components: `CodeEditor` (lazy Monaco wrapper), `LanguageSelect`,
`ProblemStatement`, `TestResultPanel`, `VerdictBadge`, `LeaderboardTable`,
`ProblemFilters`.

Hooks: `usePracticeSocket` — **one WebSocket for the whole application, in a
context provider.** It owns the auth handshake, exponential-backoff reconnect,
and a `subscribe(type, handler)` API. A socket mounted inside the workspace page
becomes a second socket as soon as anything else wants events, and a reconnect
storm when a user moves between problems.

`useSubmission(id)` listens for `submission_update` and falls back to polling
`GET /submissions/{id}` when the socket is disconnected. The page behaves
identically either way, just less immediately.

Draft source is persisted to `localStorage` per `(problem, language)`. Losing
twenty minutes of work to an accidental refresh is the fastest way to make
people abandon the feature.

On mobile the panes stack and problems are readable, but editing on a phone is
treated as out of scope rather than optimised.

## 15. Configuration

New environment variables, added to `.env.example`:

```
JUDGE0_URL=http://127.0.0.1:2358
JUDGE0_AUTH_TOKEN=
JUDGE_MAX_INFLIGHT=4
JUDGE_WORKER_TICK_MS=250
JUDGE_BATCH_POLL_START_MS=100
JUDGE_BATCH_POLL_MAX_MS=1000
JUDGE_MAX_ATTEMPTS=3
JUDGE_STALE_CLAIM_SECONDS=300
JUDGE_BATCH_DEADLINE_SECONDS=60
PRACTICE_SUBMIT_RATE_PER_MINUTE=12
PRACTICE_SUBMIT_RATE_PER_HOUR=60
PRACTICE_RUN_RATE_PER_MINUTE=30
PRACTICE_MAX_CONCURRENT_RUNS=4
PRACTICE_MAX_SOURCE_BYTES=65536
LEADERBOARD_BROADCAST_INTERVAL_MS=1000
LEADERBOARD_TOP_N=50
```

New Cargo features and dependencies: `salvo` gains the `websocket` feature;
`futures-util` for the socket stream.

## 16. Testing strategy

**Unit, no I/O**
- Verdict aggregation: every Judge0 status combination maps to the expected
  verdict, including first-failure-by-ordinal ordering and the inferred
  memory-limit reclassification.
- Output comparison: trailing whitespace and newline handling.
- Entry-point guards for Java and Kotlin.
- Scoring arithmetic and rank tie-breaking.

**Database integration**
- Award-once: two concurrent accepted submissions for the same
  `(user, problem)` produce exactly one solve row and one score increment.
- `SKIP LOCKED` claim: two workers never claim the same row.
- Stale-claim reaper returns abandoned `running` rows to `queued`.
- Rank query returns correct dense ranking with ties broken by `last_solved_at`.

**API**
- **Hidden test cases are absent from every non-admin response.** Explicit
  regression test on `GET /problems/{slug}`, `GET /submissions/{id}`, and the
  run endpoint.
- `GET /submissions/{id}` returns 403 for a non-owner non-admin.
- Rate limits return 429.
- Oversized source is rejected before insert.

**Judge integration — against a live Judge0**
- One accepted round trip per language, all 14, using that language's starter
  template. This is what proves the templates are real.
- The adversarial suite from §13.
- Judge0 unreachable: submission returns to `queued`, then `failed` after
  `JUDGE_MAX_ATTEMPTS`, and no points are awarded.

**Frontend**
- `useSubmission` produces identical final state via socket and via polling.
- Monaco is absent from the initial bundle (build-output assertion).

## 17. Rollout phases

Each phase is independently useful and independently shippable.

1. **Judge foundation.** Judge0 stack deployed and reachable; host cgroup
   requirement resolved; `judge.rs` client; boot-time language validation; all
   14 starter templates verified; adversarial suite passing. No user-facing
   surface. *This phase carries essentially all the risk in the feature, which
   is why it is first.*
2. **Catalog and schema.** Seven migrations, entities, `catalog.rs`, browse and
   detail routes, admin authoring routes, seed problems.
3. **Submission pipeline.** `problem_submissions`, worker, reaper, scoring,
   submit/run/status routes. Verdicts retrievable by polling.
4. **Frontend arena.** Browse page, workspace with Monaco, run and submit,
   results panel, submission history, `localStorage` drafts.
5. **Ranking and leaderboard.** `user_rankings`, leaderboard routes, leaderboard
   page — **polling first**.
6. **Real-time.** WebSocket hub, handler, provider, `submission_update` and
   `leaderboard_update`, coalescing and resync. Purely an upgrade to phases 4
   and 5; nothing else depends on it.

## 18. Risks

| Risk | Mitigation |
|---|---|
| Host cannot run privileged containers or cgroup v1 | Verified in phase 1 before any other work. If it fails, the hosted-judge option from the original decision reopens. |
| A starter template does not compile on the real Judge0 image | One integration test per language in phase 1; a failing language ships disabled rather than broken. |
| Judge capacity exhausted by a class of users submitting at once | `JUDGE_MAX_INFLIGHT` caps in-flight judging; the queue absorbs the rest; rate limits bound per-user pressure. |
| Hidden test case leakage | Entities never serialised directly; dedicated regression tests; a single admin-guarded route as the sole exception. |
| Judge0 language ids drift on upgrade | Boot-time validation against `GET /languages`. |
| Leaderboard broadcast storms | One-second coalescing; bounded channel; resync instead of replay. |
