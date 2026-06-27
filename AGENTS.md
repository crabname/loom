# api-helper

Desktop HTTP client (Postman-like) on Rust + GPUI (Zed stack).

**Product architecture plan** (hybrid local-cloud client, glossary, RBAC, sharing) — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Stack

- **Rust** edition 2024
- **GPUI** — UI framework from [zed-industries/zed](https://github.com/zed-industries/zed) (git rev in `Cargo.toml`)
- **gpui-component** — local path dependency: `../gpui-component/crates/ui`
- **reqwest** 0.12 (`rustls-tls`, no default-features) + **tokio**
- **anyhow** — top-level errors (HTTP layer currently uses `Result<_, String>`)

## Module Architecture

```
main.rs              — entry point: gpui_platform::application(), init, open_window
src/app/mod.rs       — logic: ApiHelperApp, state, tabs, HTTP dispatch
src/app/tab.rs       — tab state: Tab, TabSource, Params/Headers/Body panels
src/app/ui/mod.rs    — Render impl, main layout, resizable panels
src/app/ui/*.rs      — UI components: sidebar, tab_bar, url_bar, request, response, fields, curl
src/domain/          — domain: HttpMethod, BodyType, Request, Collection, fields, curl, demo
src/transport/http.rs — network: build_url_with_params, send_http_request, HttpResponse
```

### Data Flow

1. User edits a request in the UI (`app/mod.rs`) → state lives in `Tab` and syncs with `Collection`.
2. Send → `cx.spawn` clones tab data → `send_http_request()` in `transport/http.rs`.
3. Result → `finish_request()` updates `Tab.response_*`.

### Responsibility Boundaries

| Task | Module |
|------|--------|
| New body type / HTTP method | `src/domain/` |
| Request logic, headers, multipart | `src/transport/http.rs` |
| Form field structures | `src/domain/fields.rs` |
| Tab state | `src/app/tab.rs` |
| Subscriptions, tabs, request dispatch | `src/app/mod.rs` |
| Layout, render, field tables | `src/app/ui/` |

**Do not mix:** HTTP logic in `app/`, UI markup in `transport/`. New UI → `src/app/ui/`, new state logic → `src/app/mod.rs`.

## Key GPUI Patterns

- `ApiHelperApp::open()` → `cx.new(|cx| Self::new(...))` → `Entity<ApiHelperApp>`
- Text inputs: `Entity<InputState>`, selects: `Entity<SelectState<...>>`
- Subscriptions: `cx.subscribe_in(&entity, window, ...)` → store in `_subscriptions`
- Async HTTP: `cx.spawn(async move |this, cx| { ... }).detach()`
- UI update after async: `this.update(cx, |app, cx| { ...; cx.notify() })`
- Render: `impl Render for ApiHelperApp` in `src/app/ui/mod.rs`

## External Dependencies

- **Do not modify** `../gpui-component` from this repository without an explicit request
- **Do not bump** git `rev` for `gpui` / `gpui_platform` without coordination
- New crates in `Cargo.toml` — only when genuinely needed

## Build and Verification

```bash
cargo check
cargo build
cargo run
```

After changes in `src/`, run `cargo check` at minimum.

## Agent Constraints

- Minimal diff; UI in `src/app/ui/`, logic in `src/app/mod.rs`
- Do not add tests or documentation unless asked
- Commits — only on explicit user request
- User-facing responses — in Russian; code and identifiers — as in the project
