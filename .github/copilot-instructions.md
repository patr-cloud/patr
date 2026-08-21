# Patr Codebase AI Instructions

## Architecture Overview

Patr is a Rust/TypeScript monorepo for a DevOps platform. The key components are:

- **`api/`** - Axum-based backend serving multiple domains (`api.patr.cloud`, `app.patr.cloud`, `registry.patr.cloud`)
- **`frontend/`** - SolidJS + SolidStart frontend (TypeScript, TailwindCSS v4, pnpm)
- **`models/`** - Shared request/response DTOs used across API, frontend bindings, CLI, and runners
- **`macros/`** - Proc macros for endpoint declarations, route definitions, and code generation
- **`runners/`** - Docker runners that connect via WebSocket to execute deployments
- **`cli/`** - CLI tool for workspace management and runner setup

## Critical Patterns

### API Endpoints (Rust)

All endpoints use `declare_api_endpoint!` macro in `models/src/api/`. This generates typed request/response structs with validation:

```rust
macros::declare_api_endpoint!(
    /// Route documentation
    CreateDeployment,
    POST "/workspace/{workspace_id}/deployment" {
        pub workspace_id: Uuid,
    },
    request_headers = {
      pub authorization: BearerToken,
    },
    authentication = {
      AppAuthentication::<Self>::ResourcePermissionAuthenticator { ... }
    },
    request = {
      #[preprocess(trim)]
      pub name: String,
    },
    response = {
      pub id: OnlyId,
    }
);
```

Route handlers go in `api/src/routes/` mirroring the domain structure.

## Frontend Architecture (SolidJS)

### Philosophy

- **Type safety first**: Design types so invalid states are unrepresentable. Use the type system to enforce correctness at compile time rather than runtime validation.
- **Component abstraction**: Build components that encapsulate styling. Never repeat Tailwind classes—if you're copying classes, create a component instead.
- **Clean, minimalistic UX**: Simple interfaces, no clutter. Every element should earn its place.

### Bindings & Type-Safe API Calls

TypeScript types in `frontend/src/bindings/` are generated from Rust models via `ts-rs`. Each endpoint generates `{Name}Request`, `{Name}Response`, `{Name}Path`, etc. Export through `bindings/index.ts`.

**Vision**: The API layer should make it impossible to call an endpoint with mismatched request/response types—similar to how the backend's `ApiEndpoint` trait guarantees correctness by construction.

### Component Guidelines

All reusable components live in `components/` and are exported through `components/index.tsx`.

**Fetch and Action**

- Authentication happens via cookies, so no need to manually attach bearer tokens in API calls
- `httpRequest()` - low-level wrapper around `fetch` that handles auth tokens, error parsing, and response typing
- `createAsyncAction()` - higher-level abstraction for async operations with built-in loading/error state
- authenticated API calls should use the `createAuthenticatedAction()` hook for built-in loading/error state and automatic token handling

**When to create a component:**

- You're about to copy-paste styling or layout logic
- A UI pattern appears more than once
- You want to enforce consistent behavior (e.g., all buttons look the same)

**Component design principles:**

- Props should use `MaybeAccessor<T>` for values that may be static or reactive
- Use `mergeProps()` for defaults, `get()` to unwrap accessors
- Encapsulate all styling within the component—callers should only pass semantic props (variant, color, disabled), not classes
- Export from `components/index.tsx` so imports stay clean

**Existing primitives** (use before creating new ones):

- `Button` (variants: Plain, Outlined, Contained)
- `Input` (with `InputType` const for type safety)
- `Table<TItem>` (generic, typed rows)
- `PageContainer/Head/Body` (page layout)

### Route Structure

- `routes/logged-in-routes/` - Auth-protected with `PageWrapper` (Sidebar + TopBar)
- `routes/logged-out-routes/` - Public auth pages with `AuthPageWrapper`
- State via `useAuthState()` and `useLastWorkspaceId()` hooks (cookie-persisted)

### Utility Patterns

- **`utils/types.ts`**: `MaybeAccessor<T>` - value or accessor for flexible component props
- **`utils/func.ts`**: `get()` - unwrap `MaybeAccessor`, `Uuid()` - strip dashes
- **`utils/color.ts`**: `Color` enum, `ButtonVariant` const for consistent theming

### Runners

Runners implement `RunnerExecutor` trait from `runners/common`. They connect to API via WebSocket, receive deployment changes, and manage containers/pods. SQLite for local state.

## Development Commands

```bash
# Backend (runs on ports 3000, 3001, 3002 in dev)
cargo api

# Frontend (port 3030)
cd frontend && pnpm dev

# Format before commits
cargo +nightly fmt
```

## Code Style

- **Tabs, not spaces** (configured in project)
- Run `cargo +nightly fmt` before PRs
- Clippy warnings treated as errors, `unsafe` code forbidden
- Use `#[instrument]` for tracing on async functions
- Wrap response data in `WithId<T>` when returning entities with IDs

## Key Files

- `config/api.sample.json` - API config template (copy to `api.json`)
- `models/src/api/` - All endpoint definitions by domain
- `macros/src/declare_api_endpoint.rs` - Endpoint macro implementation
- `frontend/src/hooks/state-hooks.tsx` - Auth state management
- `runners/common/src/executor.rs` - Runner trait definition
