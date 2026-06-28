use gpui::{actions, App, Menu, MenuItem};
use gpui_component::{menu::AppMenuBar, GlobalState};

actions!(loom, [OpenWorkspace, OpenSettings, ImportCollection]);

pub fn register_app_menus(cx: &mut App) {
    cx.set_menus(vec![Menu {
        name: "File".into(),
        items: vec![
            MenuItem::action("Open Workspace...", OpenWorkspace),
            MenuItem::action("Import Collection...", ImportCollection),
            MenuItem::separator(),
            MenuItem::action("Settings...", OpenSettings),
        ],
        disabled: false,
    }]);

    GlobalState::global_mut(cx).set_app_menus(vec![Menu {
        name: "File".into(),
        items: vec![
            MenuItem::action("Open Workspace...", OpenWorkspace),
            MenuItem::action("Import Collection...", ImportCollection),
            MenuItem::separator(),
            MenuItem::action("Settings...", OpenSettings),
        ],
        disabled: false,
    }
    .owned()]);
}

pub fn new_app_menu_bar(cx: &mut App) -> gpui::Entity<AppMenuBar> {
    AppMenuBar::new(cx)
}
