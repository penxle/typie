# AGENTS.md

## Project Structure

Turborepo monorepo using pnpm as package manager.

- `apps/`: api, website, desktop, mobile, eval, bmo, literoom, caddy
- `packages/`: adapter-node, lib, lintconfig, prism, prism-ui, styled-system, tsconfig, ui
- `crates/`: Rust — `editor-*` 19 (bindgen, clipboard, codec, codec-macros, commands, common, core, crdt, ffi, introspection, macros, model, renderer, resource, server, state, transaction, view, xml), `prism-ui-*` 2 (renderer, web), `workspace-hack`

Per-crate responsibilities, request flow, and local setup: `docs/ONBOARDING.md`.

`apps/mobile` is Kotlin Multiplatform + Compose (with Swift for iOS), not Dart.

## Commands

```bash
pnpm install             # Install dependencies
pnpm run dev             # api, website, eval, caddy (desktop is `pnpm --filter @typie/desktop run dev:desktop`)
pnpm run build           # Build all packages
pnpm run test            # JS/TS tests only — see "Testing Rust/Kotlin" below
pnpm run lint:eslint     # Lint with ESLint
pnpm run lint:prettier   # Check formatting
pnpm run lint:typecheck  # TypeScript type checking
pnpm run lint:svelte     # Svelte-specific linting
pnpm run lint:spellcheck # Spell check with cspell
pnpm run lint:syncpack   # Check dependency version sync
```

`apps/api` and `apps/website` dev require Doppler (`doppler.yaml`) and the company tailnet; `apps/caddy` proxies :4100/:4200/:4300 to :4000.

## Rust / Native Builds

Build entry points are justfiles, not pnpm:

```bash
cd crates/editor-ffi
just wasm-browser # WASM for apps/website
just wasm-server  # WASM for apps/api (SSR)
just mobile       # UniFFI bindings + ICU for Android/iOS
```

`crates/prism-ui-web` has its own `just wasm-browser`.

## Testing Rust/Kotlin

`pnpm run test` (turbo) covers only apps/api, apps/eval, apps/website, packages/lib, packages/prism, packages/prism-ui. CI (`ci.yml`) runs the six JS/TS linters and nothing else. Rust and Kotlin have no CI gate — run them locally before claiming a change is verified:

```bash
cargo test --workspace # or -p <crate> while iterating
cargo clippy --workspace --all-targets
```

## Git Workflow

Branch and PR work goes through `gh stack` (the `github/gh-stack` CLI extension), not raw `git branch`/`push`. Staging and committing use plain `git add` / `git commit`.

```bash
gh stack init <branch>  # start a stack
gh stack add <branch>   # add a layer on top
gh stack submit --auto  # push branches and open PRs
gh stack sync           # fetch, rebase, push
gh stack view --json    # inspect stack state
```

`--auto` and `--json` are mandatory: without them the commands open interactive prompts or a TUI and hang. `remote.pushDefault` and `rerere.enabled` are already configured in this repo.

## Git Hooks (husky + lint-staged)

Pre-commit runs automatically (`lint-staged.config.js`): eslint fix, prettier fix, cspell, cargo fmt, ktfmt.
Run `pnpm run bootstrap` to install hooks after fresh clone.

## Code Style

- TypeScript: `type` for types, named exports only, `verbatimModuleSyntax`
- Formatting: 2 spaces, 140 char width, single quotes
- Naming: `kebab-case.ts` utilities, `PascalCase.svelte` components, `SCREAMING_SNAKE_CASE` constants
- Svelte: `$props()`, `$state()`, `$derived()` (Svelte 5 runes)
- Rust: stable toolchain, Edition 2024, `cargo fmt` before commits
- Rust dependencies: after changing dependencies in any `Cargo.toml`, run `cargo hakari generate` (`crates/workspace-hack` pins third-party features so different `-p`/test selections share one artifact set; do not edit it by hand)
