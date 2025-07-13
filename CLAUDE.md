# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Language

All responses should be written in Korean (한국어).

## Project Overview

Typie is a Korean writing platform that aims to be a "space for writing thoughts" and provides real-time collaboration, canvas drawing, and rich text editing features, supporting both web and mobile.

## Tech Stack & Architecture

### Monorepo Structure

- **Package Manager**: pnpm (v10.12.4) with workspaces
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

### PandaCSS Token Usage

```typescript
// Correct usage
css({ paddingX: '16px', paddingY: '8px', color: 'text.primary', backgroundColor: 'surface.primary', lineHeight: '[1.6]' });

// Incorrect usage (hardcoded values, value without unit, shorthands, multiple values, arbitrary values without brackets)
css({ p: '16 8', color: '#000000', bg: 'white', lineHeight: '1.6' });
```
