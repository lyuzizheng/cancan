use crate::runtime::VaultRuntime;
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, Window, WindowEvent};

const BACKGROUND_WINDOW_LABEL: &str = "background";
const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) fn setup_background_window(app: &mut tauri::App) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let blank =
            WebviewUrl::External(Url::parse("about:blank").expect("about:blank is a valid url"));
        tauri::webview::WebviewWindowBuilder::new(app, BACKGROUND_WINDOW_LABEL, blank)
            .visible(false)
            .decorations(false)
            .resizable(false)
            .inner_size(1.0, 1.0)
            .skip_taskbar(true)
            .build()?;
    }
    Ok(())
}

pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    #[cfg(target_os = "macos")]
    {
        if window.label() != MAIN_WINDOW_LABEL {
            return;
        }
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.destroy();
        }
    }
}

pub(crate) fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { .. } => {
            let _ = app.state::<VaultRuntime>().lock();
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => {
            let _ = show_or_create_main_window(app);
        }
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn show_or_create_main_window(app: &AppHandle) -> tauri::Result<()> {
    use tauri::webview::WebviewWindowBuilder;
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show()?;
        return window.set_focus();
    }
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or(tauri::Error::WindowNotFound)?;
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    window.set_focus()
}
