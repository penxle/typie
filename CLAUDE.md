# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language Guidelines

- All chat responses should be written in Korean (한국어).
- When adding memories to this file, always use English language.

## Tool Execution Policy

**CRITICAL: Sequential Tool Execution Required**

- All tools MUST be executed sequentially, one at a time
- Wait for each tool's result before proceeding to the next tool call
- NEVER invoke multiple tools in parallel, even if they appear independent
- This applies to ALL tools without exception: Read, Write, Edit, Bash, Grep, etc.
- If a task requires multiple tool calls, break them into separate sequential messages

This policy ensures system stability and prevents race conditions in the development environment.

## Project Overview

Typie is a Korean writing platform that aims to be a "space for writing thoughts" and provides real-time collaboration, canvas drawing, and rich text editing features, supporting both web and mobile.

## Tech Stack & Architecture

### Monorepo Structure

- **Package Manager**: bun with workspaces
- **Main directories**:
  - `apps/` - Applications (api, website, mobile, etc.)
  - `packages/` - Shared packages

### Backend (apps/api)

- **Framework**: Hono
- **GraphQL**: GraphQL Yoga + Pothos
- **Database**: PostgreSQL with Drizzle ORM
- **Real-time**: Yjs, Redis PubSub, WebSocket
- **Queue**: BullMQ
- **Search**: Meilisearch
- **Auth**: Custom OIDC provider with JWT

### Frontend (apps/website)

- **Framework**: SvelteKit + Svelte 5
- **Styling**: PandaCSS
- **Editor**: TipTap (ProseMirror)

### Mobile (apps/mobile)

- **Framework**: Flutter/Dart
- **GraphQL**: Ferry client

#### Dart/Flutter Guidelines

- **Import Order**: NEVER add import statements before writing the code that uses them. The linter automatically removes unused imports, so imports added before their usage will be deleted immediately. Always follow this order: (1) write the code that needs the import, (2) then add the import statement.

### Styling Guidelines

#### PandaCSS Token Usage

```typescript
// Correct usage
css({ paddingX: '16px', paddingY: '8px', color: 'text.default', backgroundColor: 'surface.default', lineHeight: '[1.6]' });

// Incorrect usage (hardcoded values, value without unit, shorthands, multiple values, arbitrary values without brackets)
css({ p: '16 8', color: '#000000', bg: 'white', lineHeight: '1.6' });
```

#### Color Tokens

- **Web (PandaCSS)**: Always use semantic color tokens defined in `packages/styled-system/src/colors.ts`
  - Examples: `text.default`, `surface.default`, `border.subtle`, `accent.brand.default`
- **Mobile (Flutter)**: Use semantic colors from `apps/mobile/lib/styles/semantic_colors.dart`
  - Access via `context.colors.textDefault` (using BuildContext extension from `lib/context/theme.dart`)
  - Example: `Icon(Icons.check, color: context.colors.textDefault)`
