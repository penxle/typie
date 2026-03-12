# AGENTS.md

## Project Structure

Turborepo monorepo using Bun as package manager/runtime.

- `apps/`: api, website, desktop, mobile, bmo, literoom, caddy
- `crates/`: editor (Rust, WASM)
- `packages/`: adapter-node, lib, ui, styled-system, tsconfig, lintconfig

## Commands

```bash
bun install             # Install dependencies
bun run dev             # Start all dev servers
bun run build           # Build all packages
bun run test            # Run all tests
bun run lint:eslint     # Lint with ESLint
bun run lint:prettier   # Check formatting
bun run lint:typecheck  # TypeScript type checking
bun run lint:svelte     # Svelte-specific linting
bun run lint:spellcheck # Spell check with cspell
bun run lint:syncpack   # Check dependency version sync
```

## Git Hooks (Lefthook)

Pre-commit runs automatically: eslint fix, prettier fix, cspell, dart fix/format, cargo fmt.
Run `bun run bootstrap` to install hooks after fresh clone.

## Code Style

- TypeScript: `type` for types, named exports only, `verbatimModuleSyntax`
- Formatting: 2 spaces, 140 char width, single quotes
- Naming: `kebab-case.ts` utilities, `PascalCase.svelte` components, `SCREAMING_SNAKE_CASE` constants
- Svelte: `$props()`, `$state()`, `$derived()` (Svelte 5 runes)
- Rust: Edition 2024, `cargo fmt` before commits
- Dart: `dart format`, `dart fix --apply` before commits (Flutter mobile app)

## Behavioral Guidelines

- **Think before coding**: State assumptions explicitly. Surface tradeoffs. If unclear, ask.
- **Simplicity first**: Minimum code that solves the problem. No speculative features or abstractions.
- **Surgical changes**: Touch only what you must. Match existing style. Remove only orphans YOUR changes created.
- **Goal-driven execution**: Transform tasks into verifiable goals with success criteria. State a brief plan for multi-step tasks.
