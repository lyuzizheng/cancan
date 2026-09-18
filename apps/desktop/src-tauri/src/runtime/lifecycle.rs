use crate::runtime::{VaultRuntime, lock_and_notify};
use std::sync::LazyLock;
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
            let runtime = app.state::<VaultRuntime>();
            if let Err(error) = lock_and_notify(app, runtime.inner()) {
                eprintln!("vault lock on exit failed: {}", error.code);
            }
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
pub(super) fn show_or_create_main_window(app: &AppHandle) -> tauri::Result<()> {
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

/// Whether a CanCan window is on screen right now.
///
/// Closing the main window destroys it rather than hiding it, so a closed
/// window answers `false` and leaves background intake results deliverable. A
/// window that exists but is not visible (hidden, or a background window) is
/// not on screen either. While a window *is* on screen the user is watching the
/// Tasks list update live, which is why the caller suppresses the notification
/// for that pass instead of delivering it.
pub(super) fn main_window_is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// The menu-bar item that reopens the window, so a closed window has a way back
/// besides the Dock icon. Its click and a notification click end in the same
/// place: the main window is rebuilt and the renderer pulls any route a click
/// recorded.
#[cfg(target_os = "macos")]
pub(crate) fn setup_background_intake_menu_bar(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let reopen = MenuItem::with_id(
        app,
        BACKGROUND_INTAKE_MENU_BAR_REOPEN_ID,
        "Open CanCan",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&reopen])?;
    let mut tray = TrayIconBuilder::with_id(BACKGROUND_INTAKE_MENU_BAR_ID)
        .tooltip("CanCan")
        .menu(&menu)
        // A left click reopens; the menu stays on the secondary click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == BACKGROUND_INTAKE_MENU_BAR_REOPEN_ID {
                let _ = show_or_create_main_window(app);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_or_create_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = menu_bar_icon() {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    Ok(())
}

/// The bundled application icon is a PNG; the tray wants pixels. The decode is
/// once per process and its buffer lives with the static, so the image borrows
/// from it for the life of the menu-bar item.
#[cfg(target_os = "macos")]
fn menu_bar_icon() -> Option<tauri::image::Image<'static>> {
    static ICON: LazyLock<Option<(Vec<u8>, u32, u32)>> = LazyLock::new(|| {
        let bytes = include_bytes!("../../icons/64x64.png");
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().ok()?;
        let mut buffer = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buffer).ok()?;
        buffer.truncate(info.buffer_size());
        Some((buffer, info.width, info.height))
    });
    let (buffer, width, height) = ICON.as_ref()?;
    Some(tauri::image::Image::new(buffer, *width, *height))
}

#[cfg(target_os = "macos")]
const BACKGROUND_INTAKE_MENU_BAR_ID: &str = "background-intake";
#[cfg(target_os = "macos")]
const BACKGROUND_INTAKE_MENU_BAR_REOPEN_ID: &str = "background-intake-reopen";
