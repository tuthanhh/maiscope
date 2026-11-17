# 03 — Server depends on `simai`; CI follows the crate

**What to build:** add `simai` to `apps/server`'s dependencies and move the
parser tests off the `engine` CI job.

**Blocked by:** 01 — extract `crates/simai`.

**Status:** done (2026-11-17)

- [x] `apps/server` depends on `simai` **without** the `bevy` feature
- [x] `cargo tree -p server | grep bevy` is empty — the check that this whole
      feature exists for
- [x] `ci.yml`: parser tests run in a job with no apt install and no Bevy cache.
      The `engine` job either shrinks to engine-only tests or disappears if it
      has none left
- [x] The comment in `ci.yml` explaining why `rust` is scoped `-p server -p shared`
      gains a line about feature unification — a future `--workspace` step would
      pull Bevy back in through `simai/bevy`
- [x] CLAUDE.md's Testing section points at `cargo test -p simai`
- [x] `docs/reference/architecture.md` gains the crate

> No server code calls the parser yet — that is
> [`community-charts` 02](../../community-charts/issues/02-upload-maidata.md).
> This ticket only proves the dependency is clean, which is cheaper to verify now
> than to discover later.

## Comments

**2026-11-17: done.** The dependency itself landed with ticket 02, because
the seeder moved onto `simai::maidata` there; this ticket proves it clean
and moves CI.

- The parser tests run in the `rust` job (`-p server -p simai`), which
  installs nothing from apt. The `engine` job stays, keeping its check name
  for `deploy.yml`'s gate, with the engine's own 16 tests.
- `rust` gains a step that fails if `cargo tree -p server` mentions Bevy, so
  the property is checked on every push, not only once.
- The scoping comment in `ci.yml` now explains feature unification, and
  CLAUDE.md's workspace commands no longer say `--workspace`; they did, and
  would have pulled `bevy_ecs` into the server's graph.
- `.dockerignore` names `crates/` as needed by the server image and keeps
  the gitignored real charts out of the build context.

Also found here: the `catalog-difficulties` and `song-public-id` code
already on master had never been through `cargo fmt`, which the `rust` job
checks. A `style: cargo fmt` commit on this branch fixes it; master fails CI
until this branch merges.
