# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Typie (타이피) is a cross-platform writing tool. Monorepo with Bun workspaces + Turbo orchestration, Cargo workspace for Rust crates.

**Stack**: SvelteKit (web/desktop), Flutter (mobile), Tauri (desktop shell), Bun + Hono + GraphQL-Yoga (API), Rust/WASM (editor core)

## Commands

```bash
# Development
bun run dev   # All dev servers via turbo
bun run build # Build all apps via turbo

# Linting (all run from root)
bun run lint:eslint     # ESLint (--max-warnings 0)
bun run lint:prettier   # Prettier check
bun run lint:spellcheck # CSpell
bun run lint:svelte     # svelte-check --fail-on-warnings
bun run lint:typecheck  # TypeScript type checking
bun run lint:syncpack   # Package version consistency

# Testing
bun run test # All tests via turbo

# Code generation (usually handled by turbo deps, but can run manually)
# Per-app: svelte-kit sync && panda codegen && sark codegen

# Rust / WASM
cargo test -p editor  # Editor crate tests
cargo bench -p editor # Editor benchmarks
# wasm:build task in turbo for WASM compilation (uses wasm-pack)

# Mobile (apps/mobile/)
bun run codegen:build # Dart FFI bindings generation
bun run codegen:watch # Watch FFI changes

# API-specific
# Dev uses doppler for secrets: doppler run -- bun run --watch src/main.ts
bun run dev:email # Email template preview (port 3001)
```

## Architecture

### Apps

- **`apps/api/`** — Backend server. Bun runtime, Hono HTTP framework, GraphQL-Yoga with Pothos schema builder. PostgreSQL via Drizzle ORM. WebSocket subscriptions via `graphql-ws`. Redis pub/sub, BullMQ job queue, Meilisearch. Entry: `src/main.ts` (port 3000). Secrets managed by Doppler.
- **`apps/website/`** — Main web app. SvelteKit + Vite, Panda CSS for styling, Sark for GraphQL client. SSR with `@typie/adapter-node`.
- **`apps/desktop/`** — Desktop app. Tauri 2 wrapping SvelteKit (static adapter). Same UI packages as website.
- **`apps/mobile/`** — Flutter app. Uses `ferry` for GraphQL, FFI bindings to Rust editor crate (via `native` feature flag).
- **`apps/literoom/`** — AWS Lambda image processing (Sharp).

### Crates (Rust)

- **`crates/editor/`** — Core editor engine. Text layout (`parley`), rendering (`tiny-skia`, `skrifa`), document model, CRDT collaboration (`loro`). Compiles to WASM (default `wasm` feature) or native Android (`native` feature with JNI). Key modules: `layout/`, `render/`, `model/`, `state/`, `transaction/`, `runtime/`, `schema/`.
- **`crates/fondue/`** — Font rendering native Node.js addon (NAPI-RS). Rust + C++ via CXX, includes WOFF2/Brotli submodules.

### Packages (shared)

- **`packages/sark/`** — Custom GraphQL client + code generator. Runtime uses exchanges pattern (fetch, websocket, cache with normalization). Vite plugin for codegen integration. SvelteKit load helpers.
- **`packages/ui/`** — Shared Svelte component library. Heavy Tiptap integration: custom marks (ruby, color), nodes (table, code block, embed, callout, fold), extensions (collaboration, clipboard, typewriter). Also: actions (focus-trap, portal), state helpers, notification system.
- **`packages/styled-system/`** — Panda CSS design system output.
- **`packages/lib/`** — Core utilities (logging via LogTape, helpers via Remeda/ts-pattern).
- **`packages/lintconfig/`** — Shared ESLint/Prettier configuration.
- **`packages/tsconfig/`** — Shared TypeScript config (strict, ESNext, bundler resolution).
- **`packages/adapter-node/`** — Custom SvelteKit Node.js adapter.

### Key Patterns

- **Codegen pipeline**: Turbo `codegen` task runs `svelte-kit sync`, `panda codegen`, `sark codegen` before build/dev. Many tasks depend on this.
- **GraphQL schema**: Pothos builder with plugins (dataloader, scope-auth, zod). Resolvers organized by domain in `apps/api/src/graphql/resolvers/`. Schema auto-written to file in dev.
- **Database**: Drizzle ORM schemas in `apps/api/src/db/schemas/`. Custom ID generation with table prefixes. JSONB columns for flexible data.
- **Collaborative editing**: Loro CRDT for conflict-free sync, Yjs + y-prosemirror for browser-side integration, transaction-based edit system.
- **Cross-platform editor**: Rust core compiled to WASM for web, native FFI for mobile (JNI on Android), Tiptap/ProseMirror as the rich-text editing layer on the frontend.

## Code Style

- TypeScript strict mode everywhere.
- `workspace:*` for internal package dependencies.
- Bun as package manager and API runtime.
- Korean UI/commit messages are common.

## Environment

- **Secrets**: Doppler CLI (`doppler run --` prefix for dev commands).
- **Local env**: `.envrc` with direnv, loads `.env.local`.
- **Git hooks**: Lefthook — pre-commit runs ESLint, Prettier, CSpell, dart fix/format, rustfmt in parallel.
