# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language Guidelines

- All chat responses should be written in Korean (한국어).
- When adding memories to this file, always use English language.

## Project Overview

Typie is a Korean writing platform that aims to be a "space for writing thoughts" and provides real-time collaboration, canvas drawing, and rich text editing features, supporting both web and mobile.

## Development Commands

### Root Level

```bash
bun install             # Install dependencies
bun run build           # Build all packages
bun run dev             # Run all dev servers
bun run test            # Run all tests
bun run lint:eslint     # ESLint check
bun run lint:prettier   # Prettier check
bun run lint:typecheck  # TypeScript check
bun run lint:svelte     # Svelte check
bun run lint:spellcheck # CSpell check
```

### Backend (apps/api)

```bash
cd apps/api
bun run dev               # Run API server (with doppler)
bun run dev:email         # Preview email templates
bunx drizzle-kit generate # Generate DB migration
bunx drizzle-kit migrate  # Run DB migration
bunx drizzle-kit studio   # Open Drizzle Studio
```

### Frontend (apps/website)

```bash
cd apps/website
bun run dev     # Run dev server (with doppler)
bun run build   # Build for production
bun run codegen # Generate PandaCSS + Sark (GraphQL)
```

### Mobile (apps/mobile)

```bash
cd apps/mobile
bun run codegen:build # Generate GraphQL + Freezed (one-time)
bun run codegen:watch # Watch mode for codegen
bun run clean         # Clean all generated files
flutter run           # Run app
```

### Rust Crates

```bash
cd crates/editor
bun run wasm:build   # Dev build (with SIMD)
bun run wasm:release # Release build
bun run assets       # Generate ICU data + copy assets
```

### Tests

```bash
bun run test                   # All tests via turbo
cd packages/sark && bun test   # Run sark tests with vitest
cd crates/editor && cargo test # Run Rust tests
```

## Tech Stack & Architecture

### Monorepo Structure

- **Package Manager**: bun with workspaces
- **Build System**: Turbo
- **Main directories**:
  - `apps/` - Applications (api, website, mobile, desktop, literoom)
  - `packages/` - Shared packages (ui, lib, styled-system, sark)
  - `crates/` - Rust crates (editor)

### Backend (apps/api)

- **Framework**: Hono
- **GraphQL**: GraphQL Yoga + Pothos
- **Database**: PostgreSQL with Drizzle ORM (schemas in `src/db/schemas/`)
- **Real-time**: Yjs, Redis PubSub, WebSocket
- **Queue**: BullMQ
- **Search**: Meilisearch
- **Auth**: Custom OIDC provider with JWT
- **Environment**: Managed by Doppler

### Frontend (apps/website)

- **Framework**: SvelteKit + Svelte 5
- **Styling**: PandaCSS
- **Editor**: TipTap (ProseMirror)
- **GraphQL Client**: Sark (custom, in-house client at `packages/sark`)

### Mobile (apps/mobile)

- **Framework**: Flutter/Dart (SDK ^3.8.0)
- **GraphQL**: Ferry client with code generation

### Rust Crates

- **crates/editor**: WASM-based editor logic (uses wasm-pack, Loro CRDT)

## Guidelines

### Dart/Flutter

- **Import Order**: NEVER add import statements before writing the code that uses them. The linter automatically removes unused imports, so imports added before their usage will be deleted immediately. Always follow this order: (1) write the code that needs the import, (2) then add the import statement.

### PandaCSS Token Usage

```typescript
// Correct usage
css({ paddingX: '16px', paddingY: '8px', color: 'text.default', backgroundColor: 'surface.default', lineHeight: '[1.6]' });

// Incorrect usage (hardcoded values, value without unit, shorthands, multiple values, arbitrary values without brackets)
css({ p: '16 8', color: '#000000', bg: 'white', lineHeight: '1.6' });
```

### Color Tokens

- **Web (PandaCSS)**: Always use semantic color tokens defined in `packages/styled-system/src/colors.ts`
  - Examples: `text.default`, `surface.default`, `border.subtle`, `accent.brand.default`
- **Mobile (Flutter)**: Use semantic colors from `apps/mobile/lib/styles/semantic_colors.dart`
  - Access via `context.colors.textDefault` (using BuildContext extension from `lib/context/theme.dart`)
  - Example: `Icon(Icons.check, color: context.colors.textDefault)`
