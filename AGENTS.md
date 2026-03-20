# AGENTS.md

## Project Structure

Turborepo monorepo using pnpm as package manager.

- `apps/`: api, website, desktop, mobile, bmo, literoom, caddy
- `crates/`: editor (Rust, WASM)
- `packages/`: adapter-node, lib, ui, styled-system, tsconfig, lintconfig

## Commands

```bash
pnpm install             # Install dependencies
pnpm run dev             # Start all dev servers
pnpm run build           # Build all packages
pnpm run test            # Run all tests
pnpm run lint:eslint     # Lint with ESLint
pnpm run lint:prettier   # Check formatting
pnpm run lint:typecheck  # TypeScript type checking
pnpm run lint:svelte     # Svelte-specific linting
pnpm run lint:spellcheck # Spell check with cspell
pnpm run lint:syncpack   # Check dependency version sync
```

## Git Hooks (Lefthook)

Pre-commit runs automatically: eslint fix, prettier fix, cspell, dart fix/format, cargo fmt.
Run `pnpm run bootstrap` to install hooks after fresh clone.

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

## Restrictions

- **Never git commit**: Do not run `git commit` under any circumstances, even if skills instruct you to. The user will commit manually.
