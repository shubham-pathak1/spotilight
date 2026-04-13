use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::{WebviewWindowBuilder},
    Manager, RunEvent, WebviewUrl, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Shared state for UI toggles
struct AppState {
    monochrome_enabled: Mutex<bool>,
    monochrome_item: Arc<CheckMenuItem<tauri::Wry>>,
    autostart_item: Arc<CheckMenuItem<tauri::Wry>>,
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
        let state = app.state::<AppState>();
        let mut enabled = state.monochrome_enabled.lock().unwrap();
        *enabled = !*enabled;
        toggle_monochrome(&w, *enabled);
        let _ = state.monochrome_item.set_checked(*enabled);
    }
}

pub fn run() {
    // Load the logo Base64 and scrub any newlines
    let logo_base64 = include_str!("../logo_base64.txt")
        .replace("\r", "")
        .replace("\n", "")
        .trim()
        .to_string();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![])))
        .setup(move |app| {
            // ── Bulletproof Inner Shell Script ───────────────────────
            // Using the Lava Lamp faces GIF!
            // No external font dependencies. Forced immediate injection.
            // Braces are escaped as {{ and }} for the Rust format! macro.
            let shell_script = format!(r#"
                (function() {{
                    const logoData = "{}";
                    
                    const inject = () => {{
                        if (document.getElementById('spotilight-shell')) return;
                        if (!document.documentElement) return;

                        const style = document.createElement('style');
                        style.id = 'spotilight-styles';
                        style.innerHTML = `
                            @import url('https://fonts.googleapis.com/css2?family=Silkscreen&display=swap');

                            #spotilight-shell {{
                                position: fixed;
                                top: 0;
                                left: 0;
                                width: 100vw;
                                height: 100vh;
                                background: #000000;
                                display: flex;
                                flex-direction: column;
                                justify-content: center;
                                align-items: center;
                                z-index: 2147483647;
                                transition: opacity 0.8s ease-in-out;
                                user-select: none;
                                pointer-events: all;
                            }}
                            #spotilight-shell.fade-out {{
                                opacity: 0;
                                pointer-events: none;
                            }}
                            .shell-logo {{
                                width: 220px;
                                height: 220px;
                                margin-bottom: 25px;
                                filter: grayscale(100%) contrast(1.2);
                                image-rendering: pixelated;
                            }}
                            .shell-text {{
                                font-family: 'Silkscreen', 'Courier New', monospace;
                                color: #ffffff;
                                font-size: 16px;
                                letter-spacing: 5px;
                                text-transform: uppercase;
                                font-weight: bold;
                                opacity: 0.8;
                            }}
                        `;
                        
                        document.documentElement.appendChild(style);
                        
                        const shell = document.createElement('div');
                        shell.id = 'spotilight-shell';
                        shell.innerHTML = '<img src="data:image/gif;base64,' + logoData + '" class="shell-logo"><div class="shell-text">just a moment...</div>';
                        document.documentElement.appendChild(shell);

                        let revealed = false;
                        const reveal = () => {{
                            if (revealed) return;
                            revealed = true;
                            shell.classList.add('fade-out');
                            setTimeout(() => {{
                                shell.remove();
                                style.remove();
                            }}, 1000);
                        }};

                        const checkReady = setInterval(() => {{
                            if (document.querySelector('[data-testid="control-button-playpause"]') || 
                                document.querySelector('.login-button') ||
                                document.querySelector('#main')) {{
                                clearInterval(checkReady);
                                reveal();
                            }}
                        }}, 200);

                        setTimeout(reveal, 10000);
                    }};

                    const interval = setInterval(() => {{
                        if (document.documentElement) {{
                            clearInterval(interval);
                            inject();
                        }}
                    }}, 10);

                    document.addEventListener('DOMContentLoaded', inject);
                }})();
            "#, logo_base64);

            // ── Create Main Window Manually ──────────────────────────
            let window_builder = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External("https://open.spotify.com".parse().unwrap())
            )
            .title("Spotilight")
            .inner_size(1100.0, 750.0)
            .min_inner_size(800.0, 600.0)
            .center()
            .initialization_script(&shell_script);
            
            let _window = window_builder.build().expect("failed to build main window");

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
            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_item = CheckMenuItem::with_id(
                app,
                "autostart",
                "Start with Windows",
                true,
                autostart_enabled,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit Spotilight", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_hide,
                    &PredefinedMenuItem::separator(app)?,
                    &monochrome_item,
                    &autostart_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;

            // Store shared state for syncing
            let monochrome_arc = Arc::new(monochrome_item);
            let autostart_arc = Arc::new(autostart_item);
            app.manage(AppState {
                monochrome_enabled: Mutex::new(false),
                monochrome_item: monochrome_arc,
                autostart_item: autostart_arc,
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
                    "autostart" => {
                        let state = app.state::<AppState>();
                        let manager = app.autolaunch();
                        if manager.is_enabled().unwrap_or(false) {
                            let _ = manager.disable();
                            let _ = state.autostart_item.set_checked(false);
                        } else {
                            let _ = manager.enable();
                            let _ = state.autostart_item.set_checked(true);
                        }
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

            // Media keys — no modifiers needed
            let shortcut_play_pause = Shortcut::new(None, Code::MediaPlayPause);
            let shortcut_next = Shortcut::new(None, Code::MediaTrackNext);
            let shortcut_prev = Shortcut::new(None, Code::MediaTrackPrevious);

            app.global_shortcut().on_shortcuts(
                [
                    shortcut_monochrome,
                    shortcut_play_pause,
                    shortcut_next,
                    shortcut_prev,
                ],
                move |app, shortcut, _event| {
                    let window = match app.get_webview_window("main") {
                        Some(w) => w,
                        None => return,
                    };

                    if shortcut.id() == shortcut_monochrome.id() {
                        do_monochrome_toggle(app);
                    } else if shortcut.id() == shortcut_play_pause.id() {
                        click_spotify_button(
                            &window,
                            r#"[data-testid="control-button-playpause"]"#,
                        );
                    } else if shortcut.id() == shortcut_next.id() {
                        click_spotify_button(
                            &window,
                            r#"[data-testid="control-button-skip-forward"]"#,
                        );
                    } else if shortcut.id() == shortcut_prev.id() {
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
