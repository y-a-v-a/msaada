# IMPROVEMENTS

Priority-ordered checklist of improvements based on code review. Items near the top are correctness/security; items lower down are structure and polish.

## Correctness & Security

- [x] **Cap POST body size.** `handle_post` (`src/main.rs:201`, `:230`, `:269`) drains `web::Payload` into a `BytesMut` with no ceiling — a single large POST can OOM the process. Add a `PayloadConfig` limit (e.g. 4 MiB default) and a CLI override.
  Done: introduced `PostSizeLimit(usize)` newtype (default 4 MiB) registered as `web::Data` on the app. `handle_post` now streams each branch (JSON / form / text / binary / multipart fields) through a shared `drain_with_limit` helper that returns `413 Payload Too Large` once the cap is exceeded. CLI override is `--max-post-size <BYTES>`. All 133 unit tests and the POST integration suite pass.

- [ ] **Gate the POST echo behind a flag; default to 405.** The catch-all `#[post("/{path:.*}")]` handler silently hijacks every POST path. Add an opt-in `--echo-post` (or similar) flag; without it, POST to any path returns `405 Method Not Allowed`. Adjust `--test` / self-test so it implies `--echo-post`.

- [x] **Fix the traversal-defense fallback in `serve_file_with_rewrites`.** `src/main.rs:417-422` falls back to the raw (possibly relative) `serve_dir` when `canonicalize()` fails, which makes the `starts_with(&canonical_root)` check at `:450` silently no-op. Canonicalize once at startup and fail fast if it errors; don't recompute per request.
  Done: `effective_serve_dir` is canonicalized once at startup and the process exits if that fails. The handler no longer re-canonicalizes per request and treats the passed path as already-canonical. The per-file containment check now propagates `canonicalize()` failures as `PermissionDenied` instead of silently skipping them. Added `tests/traversal_defense.rs` with a raw-TCP client (so `..` segments reach the server verbatim) covering literal and percent-encoded traversal plus a symlink-escape case.

- [x] **Validate `--port`.** `src/main.rs:743` does `.parse::<u16>().unwrap()` — non-numeric input panics with a backtrace. Use `clap::value_parser!(u16).range(1..)` and let clap produce a friendly error.
  Done: `--port` now uses `clap::value_parser!(u16).range(1..)` and the `unwrap`-based string parse is gone. Bad input (`abc`, `0`, `65536`) exits with a clear `error: invalid value '...' for '--port <port>': ...` message and no backtrace.

- [ ] **Encode TLS format invariants in the type.** `src/tls.rs:138` `self.key_path.as_ref().unwrap()` relies on a "validated in `from_args`" comment. Replace `CertFormat` + optional `key_path` with an enum whose variants carry their own data (`Pem { cert, key }`, `Pkcs12 { cert, passphrase }`). The unwrap disappears.

## Dependencies

- [x] **⚠️ URGENT: plan rustls 0.21 → 0.23 and replace `p12`.** Rustls 0.21 is EOL; `p12 = "0.6"` is unmaintained. This is next on the list after the correctness fixes above. Breaking API changes to expect:
  - `rustls::Certificate` / `PrivateKey` → `rustls-pki-types::CertificateDer` / `PrivateKeyDer`
  - `actix-web` feature flag: `rustls-0_21` → `rustls-0_23`
  - `rustls-pemfile` 1.0 → 2.x
  - For PKCS#12: evaluate `p12-keystore` or use `rustls-native-certs` where possible.
  Treat it as a single focused PR — don't drift.
  Done: rustls 0.23 (ring provider, no aws-lc-rs), `rustls-pemfile` 2, `tokio-rustls` 0.26, `p12-keystore` replacing `p12`, `bind_rustls_0_23`. All suites pass except the pre-existing `pkcs12_certificate_support` integration test, which was broken before the upgrade (test helper's `NamedTempFile::new()` produces no `.p12` extension, so msaada's format detector falls back to PEM).

- [ ] **Drop `env_logger`.** `src/main.rs:889` initializes `env_logger` after the custom logger is already in use. The two loggers are redundant and produce overlapping output. Remove `env_logger` and `log` usage (or keep `log` as a facade for internal modules but don't init two sinks).

## Structure

- [ ] **Split `main.rs`.** 1,097 lines holding CLI spec, middleware, POST handler, file handler, and `main()`. Suggested split:
  - `cli.rs` — the `clap::Command` builder
  - `handlers/post.rs` — `handle_post`, multipart/json/form parsing
  - `handlers/files.rs` — `serve_file_with_rewrites`, `normalize_request_path`, `is_hidden_path`
  - `middleware.rs` — `CustomLogger` transform
  - `main.rs` — orchestration only

- [ ] **Remove `env::set_var("RUST_LOG")` at `src/main.rs:610`.** Stop overwriting user env vars; just configure your own logger directly. (Also becomes `unsafe` in edition 2024.)

- [ ] **Remove `env::set_current_dir(dir)` at `src/main.rs:749`.** Action-at-a-distance; forces every downstream path to reason about "which directory am I actually in?". Keep paths explicit via `serve_dir`/`effective_serve_dir` and never `chdir`.

- [ ] **Simplify `config.rs` public-dir resolution.** The public path is resolved to absolute at `src/config.rs:240-250`, then re-resolved with the same relative/absolute check inside `validate_config`. Pick one location and one representation.

- [ ] **Drop `now.json` support.** Deprecation warning already prints; at 0.x with `publish = false`, just remove it. Keep `serve.json` and `package.json[static]`.

## Dead Code

- [ ] **Wire up `ShutdownManager`; delete `setup_basic_signal_handling`.** `src/shutdown.rs` has ~120 lines of `ShutdownManager` wrapped in `#[allow(dead_code)]`, while `main` uses the simpler helper. Finish the integration: pass `HttpServer::run()`'s `ServerHandle` into the manager, let it own signal handling and the force-exit timeout, and remove the `#[allow(dead_code)]` attributes.

- [ ] **Fix the `force_shutdown` capture bug.** `src/shutdown.rs:83-88` (and the mirror in `setup_basic_signal_handling`) captures a plain `bool` by move into a spawned task; the inner `if force_shutdown` check is always `true`. Either make it `Arc<AtomicBool>` for real shared state, or remove the misleading check and unconditionally exit after the timeout.

- [ ] **Delete `spa_fallback_handler` in `src/spa.rs:11`.** Marked `#[allow(dead_code)]` with a "backward compat" note; no public API to preserve.

- [ ] **Delete `apply_url_rewrites` helper in `src/spa.rs:138`.** Lives in the test module only; the real rewrite engine is in `rewrite.rs`. Update tests to use the real engine.

## Rewrite Engine

- [ ] **Replace hand-rolled rewrite code with crates.**
  - Add `globset` (or `wax`) and use it to turn glob patterns (`*`, `**`, `?`, `{a,b}`) into regex. Delete `expand_braces` and the glob-handling branches of `pattern_to_regex`.
  - Use `regex::Captures::expand` for `$1`/`${1}` substitution. Delete `substitute_captures` and `has_substitution_pattern`.
  - Keep named-param (`:name`) → `(?P<name>[^/]+)` conversion, but isolate it behind one small function.
  - Kill the "does this look like regex?" heuristic at `src/rewrite.rs:177-183` — treat input uniformly as glob+named-params and let users opt into raw regex via an explicit config field if ever needed.

- [ ] **Cache compiled regexes and match logic in one place.** After the rewrite above, `match_rewrite` becomes a straight loop over a `Vec<CompiledRewrite>` with no branching on `has_substitution`.

## Ergonomics

- [ ] **Make `logger::get_logger()` panic if not initialized** (`src/logger.rs:246`). The silent `DEFAULT_LOGGER` fallback ignores `--no-timestamps` if any code path logs before `init_logger`. Change to `.expect("logger not initialized")`; fix any call sites that trip it.

- [ ] **Self-test endpoint: remove the one-shot `AtomicBool` guard.** `src/main.rs:307` prevents re-running the test without a server restart, which is user-hostile. Either make it re-runnable or remove the flag entirely.

- [ ] **Tighten CORS defaults.** `Cors::permissive()` means "allow everything." Keep it as default when `--cors` is set, but document it and consider reading an allowlist from `serve.json` (`headers` config already exists — extend it).

## Testing

- [ ] **Move large test modules into `tests/`.** `config.rs` (~360 lines of tests) and `rewrite.rs` (~550 lines) would read better as integration tests under `tests/`. Shrinks the source files and clarifies what's public API.

- [x] **Add a traversal-defense test.** After fixing the canonicalization fallback, add a test that requests `../../etc/passwd` (and URL-encoded variants) and asserts 403/404, not file contents.
  Done together with the canonicalization fix — see `tests/traversal_defense.rs`.
