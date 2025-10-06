# Rust Code Review

This review covers the current contents of the `src/` directory with a focus on
idiomatic Rust usage, maintainability, and correctness.

## Summary

The project offers a rich feature set, but several parts of the Rust codebase
would benefit from additional hardening and idiomatic polish. The most pressing
issues are related to filesystem safety, configuration defaults, and
performance/correctness edge cases in the networking utilities. The sections
below provide concrete action points.

## Recommended Action Points

1. **Harden path handling in `serve_file_with_rewrites`.**
   * `req.path()` is concatenated directly onto the serving directory without
     normalising or validating traversal segments, which allows requests such as
     `/../Cargo.toml` (or crafted rewrite destinations) to escape the intended
     root. Canonicalise the resolved path and ensure it still lives under the
     serve directory before attempting to open it. 【F:src/main.rs†L383-L417】

2. **Avoid cloning large rewrite tables per request.**
   * The Actix factory currently clones the entire `Vec<CompiledRewrite>` twice:
     once per worker at start-up and again for every request via
     `rewrites_clone.clone()`. Wrap the compiled rules in an `Arc<[...]>` (or
     share them via `web::Data`) so that requests only clone a cheap pointer.
     【F:src/main.rs†L388-L405】【F:src/main.rs†L888-L907】

3. **Respect existing `RUST_LOG` configuration.**
   * `main` unconditionally overwrites `RUST_LOG`, then later checks whether the
     variable was set. Detect whether the variable exists first and only supply a
     default if it was absent. 【F:src/main.rs†L482-L489】【F:src/main.rs†L748-L754】

4. **Generalise the self-test endpoint to the active listener.**
   * The `/self-test` handler posts to `http://localhost:3000/...` regardless of
     the configured port or protocol, so it fails whenever the server binds to a
     different port or to TLS. Pass the actual bind information into the handler
     or expose it via shared state. 【F:src/main.rs†L320-L374】

5. **Fix `NetworkUtils::find_available_port` overflow.**
   * The range `(start_port..(start_port + 100))` will panic in debug builds when
     `start_port` is close to `u16::MAX`. Work in a wider integer domain (e.g.
     `u32`) and clamp the upper bound to `u16::MAX`. 【F:src/network.rs†L25-L95】

6. **Prefer modern panic formatting in tests.**
   * Tests still use `panic!("...", value)` which has been soft-deprecated in
     favour of `panic!("...", value)` or the more idiomatic
     `panic!("...", value)`. Update the remaining occurrences in
     `clipboard.rs` and `shutdown.rs`. 【F:src/clipboard.rs†L208-L218】【F:src/shutdown.rs†L203-L217】

7. **Consider consolidating logging.**
   * The application mixes a bespoke `Logger` with `env_logger`, which can lead
     to duplicated or inconsistent output. Decide on one logging surface (or at
     least document the split responsibilities) to keep configuration clearer.
     【F:src/logger.rs†L1-L199】【F:src/main.rs†L732-L775】

Addressing these items will make the implementation safer and closer to
idiomatic Rust while preserving the project's extensive feature set.
