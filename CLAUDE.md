# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Language

**All responses should be in Korean (한국어).** Claude should communicate with users in Korean throughout the entire conversation.

## Overview

Typie is a writing platform that provides a personal writing space with real-time collaboration capabilities. The project uses a monorepo structure managed by pnpm workspaces and Turbo.

## Architecture

### Tech Stack

**Backend (API)**

- Node.js with TypeScript, Hono web framework
- GraphQL API using Yoga + Pothos (code-first schema)
- PostgreSQL with Drizzle ORM
- Redis for caching/pub-sub, BullMQ for job queues
- Meilisearch for full-text search
- Yjs for real-time collaboration

**Frontend (Website)**

- SvelteKit 5 with TypeScript
- PandaCSS for styling
- TipTap editor with Yjs integration
- GraphQL client with code generation

**Mobile**

- Flutter/Dart with Ferry GraphQL client
- Native integrations for social logins and payments

**Infrastructure**

- AWS infrastructure managed with Pulumi
- Docker containers deployed to ECS
- CloudFront CDN, RDS PostgreSQL, ElastiCache Redis

### Key Directories

```
apps/
├── api/          # GraphQL API server
├── website/      # SvelteKit web app
├── mobile/       # Flutter mobile app
└── infrastructure/ # Pulumi IaC

packages/         # Shared packages
├── lib/          # Common utilities
├── sark/         # GraphQL codegen tool
└── adapter-node/ # SvelteKit adapter

crates/fondue/    # Rust font processing library
```

## Common Commands

### Development

```bash
# Install dependencies and setup
pnpm install
pnpm bootstrap # Sets up Husky and Doppler

# Start development servers
pnpm dev # Runs all dev servers via Turbo

# Start specific apps
cd apps/api && pnpm dev     # API server
cd apps/website && pnpm dev # Website
```

### Building

```bash
# Build all packages
pnpm build

# Build specific app
cd apps/api && pnpm build
cd apps/website && pnpm build
```

### Code Quality

```bash
# Run all linting
pnpm lint:eslint     # ESLint
pnpm lint:prettier   # Prettier formatting
pnpm lint:spellcheck # Spell checking
pnpm lint:svelte     # Svelte-specific checks
pnpm lint:typecheck  # TypeScript type checking
pnpm lint:syncpack   # Package.json consistency

# Dart/Flutter specific linting and formatting
cd apps/mobile
flutter analyze # Static analysis for Dart/Flutter code
dart format     # Format Dart code (equivalent to Prettier for Dart)
dart fix        # Fix Dart code (equivalent to ESLint fix for Dart)

# Run tests
pnpm test # Runs tests via Turbo (currently only in sark package)
```

### Database

```bash
cd apps/api

# Generate migrations
pnpm drizzle-kit generate

# Run migrations
pnpm drizzle-kit migrate

# Database studio
pnpm drizzle-kit studio
```

### GraphQL Development

```bash
# Generate GraphQL types (run from website or mobile)
cd apps/website && pnpm codegen

# The API uses code-first schema generation with Pothos
# Schema is auto-generated when the API runs
```

### Mobile Development

```bash
cd apps/mobile

# Install Flutter dependencies
flutter pub get

# Run code generation
flutter pub run build_runner build

# Run on iOS/Android
flutter run

# Build for release
flutter build ios
flutter build apk
```

## Development Workflow

1. **Environment Variables**: Managed via Doppler. Run `pnpm bootstrap` to set up.

2. **Code Generation**: Many parts of the codebase rely on code generation:
   - GraphQL types (via sark)
   - PandaCSS styles
   - Flutter/Dart models
   - Database types (Drizzle)

3. **Real-time Collaboration**: The editor uses Yjs for collaborative editing. WebSocket connections are handled through GraphQL subscriptions.

4. **Background Jobs**: BullMQ processes async tasks like sending emails, indexing search content, and handling subscriptions.

5. **Search**: Content is indexed in Meilisearch. See `PostIndexJob` for how posts are indexed.

## Key Patterns

### GraphQL Resolvers

Located in `apps/api/src/graphql/resolvers/`. Each resolver file handles a specific domain (auth, posts, users, etc.).

### Database Schema

Defined in `apps/api/src/db/schemas/`. Uses Drizzle ORM with PostgreSQL.

### Frontend State

The website uses SvelteKit's built-in stores and GraphQL for server state. Mobile uses Ferry for GraphQL state management.

### Authentication

JWT-based auth with social login support (Google, Apple, Kakao, Naver). Tokens are stored securely and passed via GraphQL context.

## Deployment

- **API**: Deployed as Docker containers to AWS ECS
- **Website**: Deployed to AWS ECS with CloudFront CDN
- **Mobile**: Published to App Store and Google Play
- **Infrastructure**: Managed with Pulumi (`pulumi up` in infrastructure directory)

## Important Notes

- Always run `pnpm codegen` after modifying GraphQL schemas or queries
- Use Turbo for running commands across the monorepo
- Follow existing code patterns and conventions
- Test database migrations locally before deploying
- Ensure proper error handling for GraphQL resolvers
- Keep sensitive data in Doppler, never commit secrets
- **PandaCSS Token Usage**:
  - Use existing design tokens: colors (brand, gray, red, green, blue with numbers 50-950), shadows (small, medium, large)
  - For non-standard CSS values, use bracket notation: `fontSize: '[48px]'`, `width: '[100dvw]'`
  - Break compound values into individual properties: `margin: '0 auto'` → `marginX: 'auto'`, `padding: '40px 20px'` → `paddingY: '40px', paddingX: '20px'`
  - Use TypeScript checking: `pnpm lint:svelte` for Svelte files, `pnpm lint:typecheck` for full project
- **ALWAYS run linting and formatting, and typechecking after modifying files**:

  ```bash
  # For TypeScript/JavaScript/Svelte files
  pnpm eslint <file_path> --fix
  pnpm prettier --write <file_path>

  # For Dart files (in apps/mobile directory)
  cd apps/mobile
  flutter analyze <file_path>  # Static analysis
  dart format <file_path>      # Code formatting
  ```

- **Use Graphite for commits and PRs**:

  ```bash
  # Create new branch and commit
  gt create <branch-name> --no-interactive --message "<commit-message>"
  gt submit --no-interactive --no-edit --publish # Creates ready-for-review PRs

  # Amend existing commit (instead of creating new PR)
  git add <files>
  gt modify --no-interactive --message "<updated-commit-message>"
  gt submit --no-interactive --no-edit --publish # Force pushes updated commit

  # Sync branches with remote (clean up merged PRs, restack branches)
  gt sync --no-interactive --force # Run before submit if stack issues occur
  ```

- **Running TypeScript scripts**: Always use Doppler for environment variables:

  ```bash
  # In each project directory (apps/api, apps/website, etc.)
  doppler run -- pnpm tsx <script-path>

  # Example:
  cd apps/api
  doppler run -- pnpm tsx scripts/test-script.ts
  ```

## GraphQL Usage Patterns in Website

### 1. Query Patterns

#### Query Declaration

```typescript
const query = graphql(`
  query DashboardLayout_Query {
    me @required {
      id
      name
      email
      ...DashboardLayout_Sidebar_user
    }
  }
`);
```

#### Query Variables

Export `_[QueryName]_Variables` function from `+page.ts` or `+layout.ts`:

```typescript
import type { DashboardSlugPage_Query_Variables } from './$graphql';

export const _DashboardSlugPage_Query_Variables: DashboardSlugPage_Query_Variables = ({ params }) => ({
  slug: params.slug,
});
```

#### Reactive Data Access

- Use `$` prefix for reactive access in Svelte components
- Queries behave like Svelte stores

```svelte
{#if $query.me}
  <h1>{$query.me.name}</h1>
{/if}
```

### 2. Mutation Patterns

#### Declaration

```typescript
const createPost = graphql(`
  mutation HomePage_CreatePost_Mutation($input: CreatePostInput!) {
    createPost(input: $input) {
      id
      entity {
        id
        slug
      }
    }
  }
`);
```

#### Direct Function Call

- **Important**: Mutations are called directly, NOT with `.mutate()` or `.load()`

```typescript
// ✅ Correct
const resp = await createPost({
  siteId: $query.me.sites[0].id,
});

// ❌ Incorrect
await createPost.mutate({ ... });
await createPost.load({ ... });
```

### 3. Subscription Patterns

#### Declaration

```typescript
const siteUpdateStream = graphql(`
  subscription DashboardLayout_SiteUpdateStream($siteId: ID!) {
    siteUpdateStream(siteId: $siteId) {
      ... on Site {
        id
      }
      ... on Entity {
        id
        state
      }
    }
  }
`);
```

#### Subscription Management

Use `$effect` with `untrack` to prevent reactive dependencies:

```typescript
$effect(() => {
  return untrack(() => {
    const unsubscribe = siteUpdateStream.subscribe({
      siteId: $query.me.sites[0].id,
    });

    return () => {
      unsubscribe();
    };
  });
});
```

### 4. Fragment Patterns

#### Fragment Declaration

```typescript
graphql(`
  fragment DashboardLayout_Sidebar_user on User {
    id
    role
  }
`);
```

#### Fragment Usage with Props

Use `fragment()` function to unwrap fragment data from props:

```typescript
import type { ComponentName_user } from '$graphql';

type Props = {
  user: ComponentName_user; // Fragment type from parent
};

let { user: _user }: Props = $props();

const user = fragment(
  _user,
  graphql(`
    fragment ComponentName_user on User {
      id
      name
      ...NestedFragment
    }
  `),
);
```

### 5. AfterLoad Pattern

#### Route-Level Checks

Export `_[QueryName]_AfterLoad` function from `+layout.ts` or `+page.ts`:

```typescript
import type { AdminLayout_Query_AfterLoad } from './$graphql';

export const _AdminLayout_Query_AfterLoad: AdminLayout_Query_AfterLoad = ({ query, event }) => {
  if (!query.me || query.me.role !== 'ADMIN') {
    redirect(302, '/home');
  }
};
```

#### Key Points

- Function name MUST match pattern: `_[QueryName]_AfterLoad`
- Runs server-side before component renders
- Access to `query` data and `event` object
- Used for auth checks, redirects, data validation

### 6. Import Patterns

#### Module Distinction

```typescript
// Global graphql function and fragment helper
import { graphql, fragment } from '$graphql';

// Route-specific generated types
import type { PageName_Query_AfterLoad } from './$graphql';
```

### 7. GraphQL Directives

#### @required Directive

Converts nullable fields to non-nullable:

```typescript
query {
  me @required {  // User! instead of User
    id
    name
  }
}
```

### 8. Type Generation

**Always run after GraphQL changes:**

```bash
pnpm codegen
```

Required when:

- Adding/modifying queries, mutations, subscriptions
- Adding/modifying fragments
- Changing GraphQL operation variables
- Adding new GraphQL operations

### 9. Common Patterns

1. **Naming Convention**: `[ComponentOrRouteName]_[OperationType]`
   - `DashboardLayout_Query`
   - `HomePage_CreatePost_Mutation`
   - `Sidebar_user` (for fragments)

2. **Data Loading Flow**:
   - Query defined in `.svelte` file
   - Variables provided via `_Variables` export
   - AfterLoad runs server-side
   - Component renders with reactive data

3. **Fragment Composition**: Pass fragments through component props

   ```svelte
   <!-- Parent component -->
   <Sidebar $user={$query.me} />
   ```

   ```typescript
   // Child component (Sidebar.svelte)
   import type { DashboardLayout_Sidebar_user } from '$graphql';

   type Props = {
     $user: DashboardLayout_Sidebar_user;
   };
   ```

4. **Error Handling**:
   - GraphQL errors handled by framework
   - Use try-catch for mutation error handling
   - TypeScript ensures type safety

### 10. Best Practices

- Use fragments for reusable data requirements
- Keep queries close to components that use them
- Use descriptive operation names
- Leverage TypeScript for type safety
- Always clean up subscriptions in effects
- Use `untrack()` to prevent unnecessary re-subscriptions

## Performance Optimization

- **Parallel Tool Usage**: Always use parallel tasks whenever possible. When multiple independent pieces of information are needed or multiple operations must be performed, batch tool calls together in a single message to optimize performance.

  Examples:

  ```
  # Good - Reading multiple files concurrently
  Read tool call 1: file1.ts
  Read tool call 2: file2.ts
  Read tool call 3: file3.ts

  # Good - Running multiple bash commands in parallel
  Bash tool call 1: git status
  Bash tool call 2: git diff
  Bash tool call 3: npm run lint

  # Bad - Sequential execution
  Read file1.ts → response → Read file2.ts → response → Read file3.ts
  ```
