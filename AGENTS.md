# api-helper

Desktop HTTP-клиент (Postman-like) на Rust + GPUI (стек Zed).

## Стек

- **Rust** edition 2024
- **GPUI** — UI-фреймворк из [zed-industries/zed](https://github.com/zed-industries/zed) (git rev в `Cargo.toml`)
- **gpui-component** — локальный path dependency: `../gpui-component/crates/ui`
- **reqwest** 0.12 (`rustls-tls`, без default-features) + **tokio**
- **anyhow** — для ошибок на верхнем уровне (HTTP-слой пока использует `Result<_, String>`)

## Архитектура модулей

```
main.rs         — точка входа: gpui_platform::application(), init, open_window
src/app/mod.rs  — логика: ApiHelperApp, состояние, вкладки, HTTP-отправка
src/app/ui.rs   — UI: Render, layout, sidebar, панели, таблицы полей
src/models.rs   — домен: HttpMethod, BodyType, Request, Collection, demo_collections()
src/http.rs     — сеть: build_url_with_params, send_http_request, HttpResponse
src/forms.rs    — типы полей: FormField, KeyValueField, MultipartField
src/tabs.rs     — состояние вкладки: Tab, TabSource, панели Params/Headers/Body
```

### Поток данных

1. Пользователь редактирует запрос в UI (`app.rs`) → состояние хранится в `Tab` и синхронизируется с `Collection`.
2. Send → `cx.spawn` клонирует данные вкладки → `send_http_request()` в `http.rs`.
3. Результат → `finish_request()` обновляет `Tab.response_*`.

### Границы ответственности

| Задача | Модуль |
|--------|--------|
| Новый тип тела / метод HTTP | `models.rs` |
| Логика запроса, заголовки, multipart | `http.rs` |
| Структуры полей форм | `forms.rs` |
| Состояние вкладки | `tabs.rs` |
| Подписки, вкладки, отправка запроса | `app/mod.rs` |
| Layout, рендер, таблицы полей | `app/ui.rs` |

**Не смешивать:** HTTP-логику в `app/`, UI-разметку в `http.rs`. Новый UI → `app/ui.rs`, новая логика состояния → `app/mod.rs`.

## Ключевые паттерны GPUI

- `ApiHelperApp::open()` → `cx.new(|cx| Self::new(...))` → `Entity<ApiHelperApp>`
- Поля ввода: `Entity<InputState>`, селекты: `Entity<SelectState<...>>`
- Подписки: `cx.subscribe_in(&entity, window, ...)` → хранить в `_subscriptions`
- Async HTTP: `cx.spawn(async move |this, cx| { ... }).detach()`
- Обновление UI после async: `this.update(cx, |app, cx| { ...; cx.notify() })`
- Рендер: `impl Render for ApiHelperApp`

## Внешние зависимости

- **Не менять** `../gpui-component` из этого репозитория без явного запроса
- **Не обновлять** git `rev` для `gpui` / `gpui_platform` без согласования
- Новые crates в `Cargo.toml` — только при реальной необходимости

## Сборка и проверка

```bash
cargo check
cargo build
cargo run
```

После изменений в `src/` запускать `cargo check` минимум.

## Ограничения для агента

- Минимальный diff; UI в `app/ui.rs`, логика в `app/mod.rs`
- Не добавлять тесты/документацию, если не просили
- Коммиты — только по явному запросу пользователя
- Ответы пользователю — на русском, код и идентификаторы — как в проекте
