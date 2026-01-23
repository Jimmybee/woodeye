use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    App, AppHandle, Emitter,
};

pub fn build_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let theme_system = CheckMenuItemBuilder::with_id("theme_system", "System")
        .checked(true)
        .build(app)?;

    let theme_light = CheckMenuItemBuilder::with_id("theme_light", "Light")
        .checked(false)
        .build(app)?;

    let theme_dark = CheckMenuItemBuilder::with_id("theme_dark", "Dark")
        .checked(false)
        .build(app)?;

    let theme_submenu = SubmenuBuilder::new(app, "Theme")
        .item(&theme_system)
        .item(&theme_light)
        .item(&theme_dark)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&theme_submenu)
        .build()?;

    let open_config = MenuItemBuilder::with_id("open_config", "Open Config File...")
        .build(app)?;

    let settings_menu = SubmenuBuilder::new(app, "Settings")
        .item(&open_config)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&view_menu)
        .item(&settings_menu)
        .build()?;

    app.set_menu(menu)?;

    Ok(())
}

pub fn update_theme_checkmarks(app_handle: &AppHandle, theme: &str) -> Result<(), String> {
    let menu = app_handle.menu().ok_or("No menu found")?;

    let update_item = |id: &str, should_check: bool| -> Result<(), String> {
        if let Some(item) = menu.get(id) {
            if let Some(check_item) = item.as_check_menuitem() {
                check_item
                    .set_checked(should_check)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    };

    update_item("theme_system", theme == "system")?;
    update_item("theme_light", theme == "light")?;
    update_item("theme_dark", theme == "dark")?;

    Ok(())
}

pub fn setup_menu_events(app: &App) {
    let app_handle = app.handle().clone();

    app.on_menu_event(move |_app, event| {
        let id = event.id().0.as_str();

        match id {
            "theme_system" | "theme_light" | "theme_dark" => {
                let theme = match id {
                    "theme_system" => "system",
                    "theme_light" => "light",
                    "theme_dark" => "dark",
                    _ => return,
                };

                if let Err(e) = update_theme_checkmarks(&app_handle, theme) {
                    eprintln!("Failed to update theme checkmarks: {}", e);
                }

                if let Err(e) = app_handle.emit("menu-theme-changed", theme) {
                    eprintln!("Failed to emit theme event: {}", e);
                }
            }
            "open_config" => {
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::commands::open_config_file().await {
                        eprintln!("Failed to open config file: {}", e);
                    }
                });
            }
            _ => {}
        }
    });
}
