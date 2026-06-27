# Environments

How environments and variables are modeled and exposed in the desktop client.

Related architecture concepts — [ARCHITECTURE.md](ARCHITECTURE.md) (glossary, variable hierarchy, workspace vs collection scope).

---

## Overview

An **environment** is a named set of **variables** (key–value pairs) used at request runtime. Environments can belong to:

| Scope | Storage (current) | Shown in selector as |
|-------|-------------------|------------------------|
| **Workspace** (global) | `Workspace.environments` | `Production`, `Staging`, … |
| **Collection** (local) | `Collection.environments` | `Demo API / JSONPlaceholder`, … |

**Target model** (aligned with [Bruno](https://github.com/usebruno/bruno)): workspace and collection environments are independent scopes with a shared variable pool at request time. Collection variables override workspace variables on name conflicts. See [Workspace vs Collection scope](#workspace-vs-collection-scope) for resolution rules and script API.

**Current UI:** a single runtime selector lists workspace environments and all collection environments (`CollectionName / EnvironmentName`). Only one environment can be active at a time (`WorkspaceSession.active_environment`). Dual activation (workspace + collection) is planned.

---

## Workspace vs Collection scope

Two environment levels differ in **reach** and **override priority**. Both can be active at the same time; their variables are merged into one runtime pool.

### Comparison

| Characteristic | Collection environment | Workspace environment (global) |
|----------------|------------------------|--------------------------------|
| **Availability** | Only inside the collection it belongs to | All collections in the workspace |
| **Priority** | Higher — wins on name conflict | Lower — overridden by collection |
| **On-disk storage** (planned) | Inside the collection folder (per-collection env files) | Workspace root (YAML) |
| **UI templates** | `{{my_var}}` | `{{my_var}}` |
| **Scripts** (planned) | `getEnvVar('var')` — collection first, then workspace fallback | Same call; global value used only when collection has no such key |

### How they work together

**1. Simultaneous activation (intersection)**

You can activate a collection environment (e.g. `Staging`) and a workspace environment at the same time. The client merges their variables into one pool.

If the workspace has `{{timeout}}` and the collection has `{{base_url}}`, requests see both variables.

**2. Name conflicts (priority)**

If the same name exists in both scopes (e.g. `{{api_key}}`), the **collection** value wins for requests in that collection. The workspace value remains available to other collections that do not define the same key.

**3. What to put where**

| Scope | Typical contents |
|-------|------------------|
| **Workspace (global)** | Cross-cutting utilities shared across services: API version, partner ID, internal feature flags |
| **Collection (local)** | Service-specific URLs and credentials: `https://users-stage.local`, database secrets for that service |

### Variable resolution order

At request runtime (URL, headers, body, scripts):

```text
1. Active collection environment variable (if key exists)
2. Active workspace environment variable (fallback)
3. Unresolved — literal `{{name}}` or script error (TBD per call site)
```

Collection scope always takes precedence when both define the same key. This matches Bruno’s merge semantics.

### Script API (intentional difference from Bruno)

Bruno exposes two separate script helpers:

| Bruno | Behavior |
|-------|----------|
| `bru.getEnvVar('var')` | Collection environment only |
| `bru.getGlobalEnvVar('var')` | Workspace / global environment only |

**api-helper** uses a single read with fallback for collection-scoped access:

| api-helper (planned) | Behavior |
|----------------------|----------|
| `getEnvVar('var')` | Active collection env → if missing, active workspace env |
| `getGlobalEnvVar('var')` | Workspace env only (optional explicit API for scripts that must ignore collection overrides) |

Rationale: most scripts need “the effective value” without choosing scope manually; fallback avoids duplicating keys in every collection while still allowing collection overrides where needed.

Template substitution `{{var}}` follows the same resolution order as `getEnvVar`.

### On-disk layout (planned)

When `LocalStorageProvider` lands:

```text
workspace/
├── environments/
│   ├── production.yml      # workspace scope
│   └── staging.yml
└── collections/
    └── users-api/
        └── environments/
            └── staging.yml # collection scope
```

Exact file names and format will follow the YAML workspace schema; collection env files live under the collection, workspace env files under the workspace root.

---

## Domain model

### `Environment` (`src/domain/environment.rs`)

```text
Environment
├── name: String
└── variables: Vec<Variable>
```

### `Variable` (`src/domain/variable.rs`)

```text
Variable
├── name: String
└── value: serde_json::Value
```

- In memory, values use `serde_json::Value` so future YAML/JSON import can store any JSON-compatible type (string, number, bool, null, array, object).
- The UI currently edits variables as **strings** only; non-string values are shown via `Variable::display_value()` as JSON text.

### References

```text
EnvironmentRef
├── scope: EnvironmentScope   // Workspace | Collection(index)
└── index: usize              // index in the scope's environments vec
```

Helper logic lives in `ApiHelperApp` (`src/app/mod.rs`): `environment_entries()`, `refresh_environment_select()`, `apply_environments_manager()`, `reconcile_active_environment()`.

---

## UI

Placement follows [Bruno](https://github.com/usebruno/bruno): a compact env bar sits in a **header row above request tabs**, right-aligned (`src/app/ui/tab_bar.rs` → `render_environment_bar()` in `environment.rs`).

```text
┌─────────────────────────────────────────────────────────────┐
│                          [Environment ▼]  [⚙ Manage]        │  ← env bar
├─────────────────────────────────────────────────────────────┤
│  GET Users  │  POST Create Post  │                    [+]   │  ← tab bar
├─────────────────────────────────────────────────────────────┤
│  method │ URL …                                    │ Send   │  ← url bar
└─────────────────────────────────────────────────────────────┘
```

### Runtime bar

| Control | Action |
|---------|--------|
| **Environment** (select) | Choose the active environment for requests (workspace + all collection envs). |
| **Manage** (⚙) | Opens the **Manage environments** dialog. |

There are no separate create / delete / configure buttons on the bar.

### Manage environments dialog

Opened via **Manage**. Two independent tabs:

| Tab | Contents |
|-----|----------|
| **Workspace** | List of workspace environments |
| **Collection** | Collection picker (when multiple collections) + list of that collection's environments |

For the selected environment in the active tab:

- **Rename** — edit the **Name** field
- **Edit variables** — key/value table (add / remove rows)
- **Add environment** / **Delete** — create or remove an entry in the current list

Changes apply on **Save**; **Cancel** discards edits.

Workspace and collection lists are edited independently; saving writes both back to in-memory workspace state.

### Sidebar / url bar

- **Sidebar**: workspace selector + collections tree only (no environment actions).
- **Url bar**: HTTP method, URL, cURL import/export, Send.

---

## Demo data

`demo_workspaces()` (`src/domain/demo.rs`):

- Workspace **Personal**: `Production`, `Staging` (with `baseUrl`, `apiKey`)
- Workspace **Local Dev**: `Local`
- Collection **Demo API**: `JSONPlaceholder` (with `baseUrl`)
- Collection **Local**: `Localhost` (with `baseUrl`)

---

## Not implemented yet

Per [ARCHITECTURE.md](ARCHITECTURE.md) variable hierarchy:

| Level | Status |
|-------|--------|
| Environment variables (model + manager UI) | ✅ |
| Dual active env (workspace + collection at once) | ❌ single selector today |
| Variable pool merge + collection-over-workspace priority | ❌ |
| `{{variable}}` substitution in URL, headers, body | ❌ |
| Script runtime + `getEnvVar` with workspace fallback | ❌ |
| Persistence (`LocalStorageProvider`, YAML on disk) | ❌ in-memory only |
| Workspace / collection / request variables (outside env) | ❌ not exposed in UI |
| Secrets / `.env.local` | ❌ |

---

## Module map

| Piece | File |
|-------|------|
| Types | `src/domain/environment.rs`, `src/domain/variable.rs`, `src/domain/workspace.rs`, `src/domain/request.rs` (`Collection.environments`) |
| State & apply | `src/app/mod.rs` |
| Session persistence | `src/app/tab.rs` (`WorkspaceSession`) |
| Env bar + manager dialog | `src/app/ui/environment.rs` |
| Layout (header above tabs) | `src/app/ui/tab_bar.rs` |
