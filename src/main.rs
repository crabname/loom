mod app;
mod demo;
mod domain;
mod import;
mod scripting;
mod storage;
mod transport;

use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;

use app::{menus, LoomApp};

fn main() {
    let app = gpui_platform::application()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.run(move |cx| {
        gpui_component::init(cx);
        menus::register_app_menus(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = LoomApp::open(window, cx);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
