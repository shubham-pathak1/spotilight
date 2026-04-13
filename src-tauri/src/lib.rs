use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// Shared state for monochrome filter toggle
struct MonochromeState {
    enabled: Mutex<bool>,
    menu_item: Arc<CheckMenuItem<tauri::Wry>>,
}

/// Inject JavaScript into the Spotify webview to click a button by selector
fn click_spotify_button(window: &tauri::WebviewWindow, selector: &str) {
    let js = format!(
        r#"
        (function() {{
            const el = document.querySelector('{}');
            if (el) el.click();
        }})();
        "#,
        selector
    );
    let _ = window.eval(&js);
}

/// Toggle monochrome CSS filter on the Spotify webview
fn toggle_monochrome(window: &tauri::WebviewWindow, enabled: bool) {
    let filter = if enabled { "grayscale(100%)" } else { "none" };
    let js = format!(
        "document.documentElement.style.filter = '{}';",
        filter
    );
    let _ = window.eval(&js);
}

/// Helper to flip monochrome state + sync tray checkmark
fn do_monochrome_toggle(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let state = app.state::<MonochromeState>();
        let mut enabled = state.enabled.lock().unwrap();
        *enabled = !*enabled;
        toggle_monochrome(&w, *enabled);
        let _ = state.menu_item.set_checked(*enabled);
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        // -- Plugins --
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // -- Setup: tray, shortcuts, window behavior --
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Show the window once Spotify starts loading
            // (window-state plugin restores position first since visible=false)
            window.show().unwrap();

            // ── System Tray ──────────────────────────────────────────

            let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide", true, None::<&str>)?;
            let monochrome_item = CheckMenuItem::with_id(
                app,
                "monochrome",
                "Monochrome Mode (Ctrl+Shift+M)",
                true,
                false,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit Spotilight", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_hide,
                    &PredefinedMenuItem::separator(app)?,
                    &monochrome_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;

            // Store monochrome state + menu item reference for syncing
            let monochrome_arc = Arc::new(monochrome_item);
            app.manage(MonochromeState {
                enabled: Mutex::new(false),
                menu_item: monochrome_arc,
            });

            let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
                .expect("failed to load tray icon");

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("Spotilight")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show_hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    "monochrome" => {
                        do_monochrome_toggle(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // ── Global Shortcuts ─────────────────────────────────────

            let shortcut_monochrome =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM);
            let shortcut_fullscreen = Shortcut::new(None, Code::F11);

            // Media keys — no modifiers needed
            let shortcut_play_pause = Shortcut::new(None, Code::MediaPlayPause);
            let shortcut_next = Shortcut::new(None, Code::MediaTrackNext);
            let shortcut_prev = Shortcut::new(None, Code::MediaTrackPrevious);

            app.global_shortcut().on_shortcuts(
                [
                    shortcut_monochrome,
                    shortcut_fullscreen,
                    shortcut_play_pause,
                    shortcut_next,
                    shortcut_prev,
                ],
                move |app, shortcut, _event| {
                    let window = match app.get_webview_window("main") {
                        Some(w) => w,
                        None => return,
                    };

                    if shortcut == &shortcut_monochrome {
                        do_monochrome_toggle(app);
                    } else if shortcut == &shortcut_fullscreen {
                        let is_fullscreen = window.is_fullscreen().unwrap_or(false);
                        let _ = window.set_fullscreen(!is_fullscreen);
                    } else if shortcut == &shortcut_play_pause {
                        click_spotify_button(
                            &window,
                            r#"[data-testid="control-button-playpause"]"#,
                        );
                    } else if shortcut == &shortcut_next {
                        click_spotify_button(
                            &window,
                            r#"[data-testid="control-button-skip-forward"]"#,
                        );
                    } else if shortcut == &shortcut_prev {
                        click_spotify_button(
                            &window,
                            r#"[data-testid="control-button-skip-back"]"#,
                        );
                    }
                },
            )?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Spotilight");

    // ── Run loop: intercept close to minimize to tray ────────────

    app.run(|app_handle, event| {
        match &event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == "main" => {
                // Don't quit — hide to tray instead
                api.prevent_close();
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            _ => {}
        }
    });
}
