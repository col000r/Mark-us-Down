use tauri::{Manager, menu::*, Emitter, WindowEvent, WebviewUrl};
use tauri::webview::WebviewWindowBuilder;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};

// Counter for generating unique window labels
static WINDOW_COUNTER: AtomicU64 = AtomicU64::new(0);

// Flag to track if a window was created from a file open event (macOS)
static FILE_OPEN_HANDLED: AtomicBool = AtomicBool::new(false);

// macOS dock menu: app handle and menu pointer stored globally
#[cfg(target_os = "macos")]
static DOCK_APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

// Raw pointer to retained NSMenu (always accessed on main thread)
#[cfg(target_os = "macos")]
static DOCK_MENU_PTR: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

// File watcher state - keyed by (window_label, file_path) to support per-window watchers
type FileWatchers = Arc<Mutex<HashMap<String, RecommendedWatcher>>>;

// Tracks which windows are currently empty (no file, no content)
type EmptyWindows = Arc<Mutex<HashSet<String>>>;

// Pending files to open, keyed by window label — set before the window loads,
// consumed by the frontend via the `window_ready` command once it has initialized.
// Newtype wrapper so Tauri's state manager sees a distinct TypeId from FileWatchers.
struct PendingFiles(Arc<Mutex<HashMap<String, (String, String)>>>);
impl Default for PendingFiles {
    fn default() -> Self { PendingFiles(Arc::new(Mutex::new(HashMap::new()))) }
}

// Windows whose frontends have fully initialized and registered their event listeners.
// Newtype wrapper so Tauri's state manager sees a distinct TypeId from EmptyWindows.
struct ReadyWindows(Arc<Mutex<HashSet<String>>>);
impl Default for ReadyWindows {
    fn default() -> Self { ReadyWindows(Arc::new(Mutex::new(HashSet::new()))) }
}

/// Generate a unique window label
fn generate_window_label() -> String {
    let count = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("doc-{}", count)
}

/// Where a new document should appear. On macOS, `Tab` joins the frontmost
/// document window's tab group; `Window` always opens a separate window.
/// Other platforms have no native tabs, so both open a window.
#[derive(Clone, Copy, PartialEq)]
enum Placement {
    Tab,
    Window,
}

/// Create a new document tab (macOS) or window, optionally with a file to open
fn create_document_window(
    app_handle: &tauri::AppHandle,
    file_path: Option<String>,
    content: Option<String>,
) -> Result<tauri::WebviewWindow, String> {
    create_document_window_with_placement(app_handle, file_path, content, Placement::Tab)
}

fn create_document_window_with_placement(
    app_handle: &tauri::AppHandle,
    file_path: Option<String>,
    content: Option<String>,
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))] placement: Placement,
) -> Result<tauri::WebviewWindow, String> {
    let label = generate_window_label();
    println!("Creating new document window with label: {}", label);

    let builder = WebviewWindowBuilder::new(app_handle, &label, WebviewUrl::App("index.html".into()))
        .title("Mark-us-Down")
        .inner_size(1200.0, 800.0)
        .min_inner_size(600.0, 400.0)
        .resizable(true)
        .center();

    // On macOS, document windows share a tabbing identifier so they can be grouped
    // into native window tabs. The window is created hidden so the tabbing mode can
    // be set before it is first ordered front (which is when AppKit decides to tab it).
    #[cfg(target_os = "macos")]
    let builder = builder.tabbing_identifier(TABBING_IDENTIFIER).visible(false);

    let window = builder
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        if placement == Placement::Window {
            // Keep AppKit from tabbing it on first show, then allow tabbing again
            // so later tabs and "Merge All Windows" still work with this window.
            set_tabbing_mode(&window, NS_WINDOW_TABBING_MODE_DISALLOWED);
            window.show().map_err(|e| format!("Failed to show window: {}", e))?;
            set_tabbing_mode(&window, NS_WINDOW_TABBING_MODE_PREFERRED);
        } else {
            set_tabbing_mode(&window, NS_WINDOW_TABBING_MODE_PREFERRED);
            window.show().map_err(|e| format!("Failed to show window: {}", e))?;
        }
        let _ = window.set_focus();
    }

    // If no file is being opened, register this window as empty
    if file_path.is_none() {
        let empty_windows: tauri::State<EmptyWindows> = app_handle.state::<EmptyWindows>();
        let mut empty_set = empty_windows.inner().lock().unwrap();
        empty_set.insert(label.clone());
        println!("Registered window {} as empty", label);
    }

    // If we have a file to open, store it as a pending file. The frontend will
    // retrieve it via the `window_ready` command once it has finished initializing.
    // This avoids a race where a fixed-delay emit fires before the listener is set up.
    if let (Some(path), Some(file_content)) = (file_path, content) {
        let pending_files: tauri::State<PendingFiles> = app_handle.state::<PendingFiles>();
        let mut pending = pending_files.inner().0.lock().unwrap();
        pending.insert(label.clone(), (path.clone(), file_content));
        println!("Stored pending file for window {}: {}", label, path);
    }

    Ok(window)
}

#[cfg(target_os = "macos")]
const TABBING_IDENTIFIER: &str = "rocks.brightlight.markusdown.document";

#[cfg(target_os = "macos")]
const NS_WINDOW_TABBING_MODE_PREFERRED: isize = 1;
#[cfg(target_os = "macos")]
const NS_WINDOW_TABBING_MODE_DISALLOWED: isize = 2;

/// Sets the NSWindow tabbing mode. Preferred makes a window open as a tab of the
/// frontmost document window, regardless of the "Prefer tabs" system setting.
#[cfg(target_os = "macos")]
fn set_tabbing_mode(window: &tauri::WebviewWindow, mode: isize) {
    use objc2::{msg_send, runtime::AnyObject};

    match window.ns_window() {
        Ok(ns_window) if !ns_window.is_null() => unsafe {
            let ns_window = ns_window as *mut AnyObject;
            let _: () = msg_send![ns_window, setTabbingMode: mode];
        },
        _ => eprintln!("Could not get NSWindow for {}; native tabs unavailable", window.label()),
    }
}

/// Builds the Window menu. Giving it WINDOW_SUBMENU_ID makes Tauri register it as
/// NSApp.windowsMenu, so AppKit adds its tab commands (Show Previous/Next Tab,
/// Move Tab to New Window, Merge All Windows) and the open-window list.
fn build_window_menu<R: tauri::Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<Submenu<R>> {
    SubmenuBuilder::with_id(manager, WINDOW_SUBMENU_ID, "Window")
        .item(&PredefinedMenuItem::minimize(manager, None)?)
        .item(&PredefinedMenuItem::maximize(manager, Some("Zoom"))?)
        .build()
}

// Tauri commands for file operations

#[tauri::command]
async fn new_file(window: tauri::Window) -> Result<(), String> {
    // Reset the file state in the current window
    window.emit_to(window.label(), "file-new", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn create_new_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Create a new empty document window
    create_document_window(&app_handle, None, None)?;
    Ok(())
}

#[tauri::command]
async fn save_file_dialog(window: tauri::Window, app_handle: tauri::AppHandle, content: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle.dialog().file()
        .add_filter("Markdown files", &["md"])
        .set_title("Save Markdown File")
        .set_file_name("untitled.md");

    let window_clone = window.clone();
    let window_label = window.label().to_string();
    dialog.save_file(move |path| {
        if let Some(path) = path {
            let path_str = path.to_string();
            let path_buf = PathBuf::from(&path_str);
            match fs::write(&path_buf, &content) {
                Ok(_) => {
                    let _ = window_clone.emit_to(&window_label, "file-saved", path_buf.to_string_lossy().to_string());
                }
                Err(e) => {
                    eprintln!("Error saving file: {}", e);
                }
            }
        }
    });

    Ok(None)
}

#[tauri::command]
async fn save_file(window: tauri::Window, path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())?;
    window.emit_to(window.label(), "file-saved", path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn read_file(window: tauri::Window, path: String) -> Result<String, String> {
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    window.emit_to(window.label(), "file-opened", (path, content.clone())).map_err(|e| e.to_string())?;
    Ok(content)
}

#[tauri::command]
async fn read_binary_file(path: String) -> Result<String, String> {
    use std::io::Read;
    let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(base64_encode(&bytes))
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        out.push(CHARS[(b0 >> 2)] as char);
        out.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        out.push(if chunk.len() > 1 { CHARS[((b1 & 15) << 2) | (b2 >> 6)] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[b2 & 63] as char } else { '=' });
    }
    out
}

#[tauri::command]
async fn update_theme_menu(app_handle: tauri::AppHandle, is_dark: bool) -> Result<(), tauri::Error> {
    println!("update_theme_menu called with is_dark: {}", is_dark);

    // Rebuild the entire menu with updated theme text
    let theme_text = if is_dark {
        "Switch to Light Mode"
    } else {
        "Switch to Dark Mode"
    };

    // Rebuild the menu
    let app_menu = SubmenuBuilder::new(&app_handle, "Mark-us-Down")
        .item(&MenuItemBuilder::new("About Mark-us-Down").id("about").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Quit Mark-us-Down").id("quit").accelerator("CmdOrCtrl+Q").build(&app_handle)?)
        .build()?;

    let file_menu = SubmenuBuilder::new(&app_handle, "File")
        .item(&MenuItemBuilder::new("New Window").id("new_window").accelerator("CmdOrCtrl+Shift+N").build(&app_handle)?)
        .item(&MenuItemBuilder::new("New Tab").id("new_tab").accelerator("CmdOrCtrl+T").build(&app_handle)?)
        .item(&MenuItemBuilder::new("New").id("new").accelerator("CmdOrCtrl+N").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Open...").id("open").accelerator("CmdOrCtrl+O").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Save").id("save").accelerator("CmdOrCtrl+S").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Save As...").id("save_as").accelerator("CmdOrCtrl+Shift+S").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Print...").id("print").accelerator("CmdOrCtrl+P").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Close").id("close").accelerator("CmdOrCtrl+W").build(&app_handle)?)
        .build()?;

    let edit_menu = SubmenuBuilder::new(&app_handle, "Edit")
        .item(&MenuItemBuilder::new("Undo").id("undo").accelerator("CmdOrCtrl+Z").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Redo").id("redo").accelerator("CmdOrCtrl+Shift+Z").build(&app_handle)?)
        .separator()
        .item(&PredefinedMenuItem::cut(&app_handle, None)?)
        .item(&PredefinedMenuItem::copy(&app_handle, None)?)
        .item(&PredefinedMenuItem::paste(&app_handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::select_all(&app_handle, None)?)
        .build()?;

    let view_menu_builder = SubmenuBuilder::new(&app_handle, "View")
        .item(&MenuItemBuilder::new(theme_text).id("theme_toggle").accelerator("CmdOrCtrl+Shift+L").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Toggle Reading Mode").id("reading_mode").accelerator("CmdOrCtrl+E").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Zoom In").id("zoom_in").accelerator("CmdOrCtrl+Plus").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Zoom Out").id("zoom_out").accelerator("CmdOrCtrl+-").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Reset Zoom").id("reset_zoom").accelerator("CmdOrCtrl+0").build(&app_handle)?);

    #[cfg(debug_assertions)]
    let view_menu_builder = view_menu_builder
        .separator()
        .item(&MenuItemBuilder::new("Debug Info").id("debug_info").build(&app_handle)?);

    let view_menu = view_menu_builder.build()?;

    let window_menu = build_window_menu(&app_handle)?;

    let menu = MenuBuilder::new(&app_handle)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .build()?;

    app_handle.set_menu(menu)?;
    println!("Menu rebuilt with theme text: {}", theme_text);

    Ok(())
}

#[tauri::command]
async fn debug_args() -> Result<Vec<String>, String> {
    let args: Vec<String> = std::env::args().collect();
    println!("Debug args called - found {} arguments:", args.len());
    for (i, arg) in args.iter().enumerate() {
        println!("  Debug Arg {}: {}", i, arg);
    }
    Ok(args)
}

#[tauri::command]
async fn start_file_watcher(window: tauri::Window, app_handle: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let watchers: FileWatchers = app_handle.state::<FileWatchers>().inner().clone();
    let window_label = window.label().to_string();

    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    // Create a unique key for this window's watcher
    let watcher_key = format!("{}:{}", window_label, file_path);

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

    watcher.watch(&path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch file: {}", e))?;

    // Store the watcher
    {
        let mut watchers_lock = watchers.lock().unwrap();
        watchers_lock.insert(watcher_key.clone(), watcher);
    }

    // Start the event loop in a separate thread
    let file_path_clone = file_path.clone();
    let window_clone = window.clone();
    let window_label_clone = window_label.clone();
    let watchers_clone = watchers.clone();
    let watcher_key_clone = watcher_key.clone();

    std::thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                    if let Event { kind: notify::EventKind::Modify(_), paths, .. } = event {
                        if paths.iter().any(|p| p.to_string_lossy() == file_path_clone) {
                            // File was modified, read new content and emit event to the specific window
                            if let Ok(content) = fs::read_to_string(&file_path_clone) {
                                let _ = window_clone.emit_to(&window_label_clone, "file-changed-externally", (&file_path_clone, &content));
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("File watcher error: {}", e);
                    // Remove watcher on error
                    let mut watchers_lock = watchers_clone.lock().unwrap();
                    watchers_lock.remove(&watcher_key_clone);
                    break;
                }
            }
        }
    });

    println!("Started watching file: {} for window: {}", file_path, window_label);
    Ok(())
}

#[tauri::command]
async fn stop_file_watcher(window: tauri::Window, app_handle: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let watchers: FileWatchers = app_handle.state::<FileWatchers>().inner().clone();
    let window_label = window.label().to_string();
    let watcher_key = format!("{}:{}", window_label, file_path);

    let mut watchers_lock = watchers.lock().unwrap();

    if watchers_lock.remove(&watcher_key).is_some() {
        println!("Stopped watching file: {} for window: {}", file_path, window_label);
        Ok(())
    } else {
        // Not an error - the watcher might have already been removed
        println!("File watcher not found for: {} in window: {}", file_path, window_label);
        Ok(())
    }
}

#[tauri::command]
async fn set_window_empty(window: tauri::Window, app_handle: tauri::AppHandle, is_empty: bool) -> Result<(), String> {
    let empty_windows: tauri::State<EmptyWindows> = app_handle.state::<EmptyWindows>();
    let mut empty_set = empty_windows.inner().lock().unwrap();
    let label = window.label().to_string();
    if is_empty {
        empty_set.insert(label.clone());
    } else {
        empty_set.remove(&label);
    }
    println!("Window {} empty state updated to: {}", label, is_empty);
    Ok(())
}

/// Called by the frontend once it has initialized and registered all event listeners.
/// Marks the window as ready and returns any file that was queued to open
/// before the frontend was available (e.g. cold-start file double-click).
#[tauri::command]
async fn window_ready(window: tauri::Window, app_handle: tauri::AppHandle) -> Result<Option<(String, String)>, String> {
    let window_label = window.label().to_string();

    // Mark as ready so future file-open events can emit directly
    {
        let ready_windows: tauri::State<ReadyWindows> = app_handle.state::<ReadyWindows>();
        let mut ready_set = ready_windows.inner().0.lock().unwrap();
        ready_set.insert(window_label.clone());
        println!("Window {} marked as ready", window_label);
    }

    // Return and clear any pending file
    let pending_files: tauri::State<PendingFiles> = app_handle.state::<PendingFiles>();
    let mut pending = pending_files.inner().0.lock().unwrap();
    let result = pending.remove(&window_label);
    if let Some((ref path, _)) = result {
        println!("Returning pending file to window {}: {}", window_label, path);
    }
    Ok(result)
}

#[tauri::command]
async fn open_file_dialog(window: tauri::WebviewWindow, app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle.dialog().file()
        .add_filter("Markdown files", &["md", "markdown", "txt"])
        .set_title("Open Markdown File");

    let window_clone = window.clone();
    let window_label = window.label().to_string();
    dialog.pick_file(move |path| {
        if let Some(path) = path {
            let path_buf = std::path::PathBuf::from(path.as_path().unwrap());
            let path_str = path_buf.to_string_lossy().to_string();
            match fs::read_to_string(&path_buf) {
                Ok(content) => {
                    let _ = window_clone.emit_to(&window_label, "file-opened", (path_str, content));
                }
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// Core file-open logic shared by RunEvent::Opened, application:openFile:, and drag-drop.
/// Reuses an empty window if one is available; otherwise creates a new document window.
fn handle_file_open(app: &tauri::AppHandle, path_str: String, content: String) {
    println!("handle_file_open: {}", path_str);

    let empty_window_label = {
        let empty_windows: tauri::State<EmptyWindows> = app.state::<EmptyWindows>();
        let mut empty_set = empty_windows.inner().lock().unwrap();
        let label = empty_set.iter()
            .find(|label| app.get_webview_window(label).is_some())
            .cloned();
        if let Some(ref l) = label {
            empty_set.remove(l);
        }
        label
    };

    if let Some(window_label) = empty_window_label {
        println!("Reusing empty window {} for file: {}", window_label, path_str);
        let is_ready = {
            let ready_windows: tauri::State<ReadyWindows> = app.state::<ReadyWindows>();
            ready_windows.inner().0.lock().unwrap().contains(&window_label)
        };
        if is_ready {
            if let Some(window) = app.get_webview_window(&window_label) {
                let wl = window_label.clone();
                let p = path_str.clone();
                let c = content.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = window.emit_to(&wl, "file-opened", (p, c));
                });
            }
        } else {
            let pending_files: tauri::State<PendingFiles> = app.state::<PendingFiles>();
            pending_files.inner().0.lock().unwrap()
                .insert(window_label.clone(), (path_str, content));
            println!("Window {} not ready yet; stored as pending file", window_label);
        }
    } else {
        println!("No empty window; creating new window for: {}", path_str);
        match create_document_window(app, Some(path_str.clone()), Some(content)) {
            Ok(_) => {},
            Err(e) => eprintln!("Failed to create window for file {}: {}", path_str, e),
        }
    }
}

/// Helper function to get the focused window or fall back to any available window
fn get_target_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    // Get all windows and find the focused one
    let windows = app.webview_windows();

    // Try to find a focused window
    for (_, window) in windows.iter() {
        if window.is_focused().unwrap_or(false) {
            return Some(window.clone());
        }
    }

    // Fall back to the first available window
    windows.into_values().next()
}

/// Clean up file watchers for a specific window
fn cleanup_window_watchers(app_handle: &tauri::AppHandle, window_label: &str) {
    let watchers: FileWatchers = app_handle.state::<FileWatchers>().inner().clone();
    let mut watchers_lock = watchers.lock().unwrap();

    // Remove all watchers that start with this window's label
    let keys_to_remove: Vec<String> = watchers_lock
        .keys()
        .filter(|k| k.starts_with(&format!("{}:", window_label)))
        .cloned()
        .collect();

    for key in keys_to_remove {
        watchers_lock.remove(&key);
        println!("Cleaned up file watcher: {}", key);
    }
}

/// Sets up the macOS dock right-click menu with a "New Window" item.
///
/// Tauri 2.x has no built-in dock menu API, so this injects two methods into
/// tao's `TaoAppDelegateParent` class using `class_addMethod`:
///   - `applicationDockMenu:` — returns the NSMenu for the dock right-click
///   - `newWindowAction:` — handles the menu item action via the responder chain
///
/// The NSMenu is built lazily inside `applicationDockMenu:` on the first
/// right-click (AppKit always calls this on the main thread), avoiding any
/// AppKit object creation during the Tauri setup phase.
#[cfg(target_os = "macos")]
fn setup_dock_menu(app_handle: &tauri::AppHandle) {
    use std::ffi::c_char;
    use objc2::{ffi as objc_ffi, runtime::{AnyClass, AnyObject}, sel};

    DOCK_APP_HANDLE.set(app_handle.clone()).ok();

    // Called when the user clicks "New Window" in the dock right-click menu.
    // Dispatches window creation through the tao event loop for safety.
    unsafe extern "C-unwind" fn new_window_action(
        _this: *mut AnyObject,
        _sel: objc2::runtime::Sel,
        _sender: *mut AnyObject,
    ) {
        if let Some(app) = DOCK_APP_HANDLE.get() {
            let app = app.clone();
            let _ = app.clone().run_on_main_thread(move || {
                if let Err(e) = create_document_window_with_placement(&app, None, None, Placement::Window) {
                    eprintln!("Failed to create window from dock menu: {}", e);
                }
            });
        }
    }

    // Called when the user clicks the "+" button in the native tab bar.
    unsafe extern "C-unwind" fn new_window_for_tab(
        _this: *mut AnyObject,
        _sel: objc2::runtime::Sel,
        _sender: *mut AnyObject,
    ) {
        if let Some(app) = DOCK_APP_HANDLE.get() {
            let app = app.clone();
            let _ = app.clone().run_on_main_thread(move || {
                if let Err(e) = create_document_window(&app, None, None) {
                    eprintln!("Failed to create tab from tab bar: {}", e);
                }
            });
        }
    }

    // Called by macOS when a file is opened via Finder "Open With" or double-click.
    // tao only implements application:openURLs:, so CFBundleDocumentTypes-based opens
    // never reach RunEvent::Opened. This fills that gap.
    unsafe extern "C-unwind" fn application_open_file(
        _this: *mut AnyObject,
        _sel: objc2::runtime::Sel,
        _sender: *mut AnyObject,  // NSApplication*
        filename: *mut AnyObject, // NSString*
    ) -> bool {
        use objc2_foundation::NSString;
        let path_str = {
            let ns_str = &*(filename as *const NSString);
            ns_str.to_string()
        };
        println!("application:openFile: received: {}", path_str);
        if let Some(app) = DOCK_APP_HANDLE.get() {
            match std::fs::read_to_string(&path_str) {
                Ok(content) => {
                    let app_clone = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        handle_file_open(&app_clone, path_str, content);
                    });
                }
                Err(e) => {
                    eprintln!("application:openFile: failed to read {}: {}", path_str, e);
                }
            }
        }
        true
    }

    // Returns the NSMenu shown when the user right-clicks the dock icon.
    // Built lazily on first call — AppKit always calls this on the main thread.
    unsafe extern "C-unwind" fn application_dock_menu(
        _this: *mut AnyObject,
        _sel: objc2::runtime::Sel,
        _app: *mut AnyObject,
    ) -> *mut AnyObject {
        use objc2::rc::Retained;
        use objc2_app_kit::{NSMenu, NSMenuItem};
        use objc2_foundation::{MainThreadMarker, ns_string};

        let existing = DOCK_MENU_PTR.load(Ordering::Acquire);
        if !existing.is_null() {
            return existing as *mut AnyObject;
        }

        // Build the menu on first call. nil target → responder chain finds
        // newWindowAction: on TaoAppDelegateParent (the app delegate).
        let mtm = MainThreadMarker::new_unchecked();
        let menu = NSMenu::new(mtm);
        let item = NSMenuItem::new(mtm);
        item.setTitle(ns_string!("New Window"));
        item.setAction(Some(objc2::sel!(newWindowAction:)));
        menu.addItem(&item);

        // Leak the menu so it lives for the app's lifetime
        let menu_raw = Retained::into_raw(menu) as *mut std::ffi::c_void;
        DOCK_MENU_PTR.store(menu_raw, Ordering::Release);
        menu_raw as *mut AnyObject
    }

    unsafe {
        if let Some(delegate_cls) = AnyClass::get(c"TaoAppDelegateParent") {
            let cls_ptr = (delegate_cls as *const AnyClass).cast_mut().cast();

            // Inject newWindowAction: so the responder chain finds it on the app delegate.
            // Type encoding: v24@0:8@16 — void return, (self id, SEL, sender id)
            let new_window_imp: objc2::runtime::Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject),
                objc2::runtime::Imp,
            >(new_window_action);
            objc_ffi::class_addMethod(
                cls_ptr,
                sel!(newWindowAction:),
                new_window_imp,
                b"v24@0:8@16\0".as_ptr() as *const c_char,
            );

            // Inject newWindowForTab: — its presence makes AppKit show the "+" button in
            // the native tab bar; clicking it opens a new document tab.
            let new_tab_imp: objc2::runtime::Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject),
                objc2::runtime::Imp,
            >(new_window_for_tab);
            objc_ffi::class_addMethod(
                cls_ptr,
                sel!(newWindowForTab:),
                new_tab_imp,
                b"v24@0:8@16\0".as_ptr() as *const c_char,
            );

            // Inject applicationDockMenu: — NSApplicationDelegate dock menu callback.
            // Type encoding: @24@0:8@16 — id return, (self id, SEL, NSApplication* id)
            let dock_menu_imp: objc2::runtime::Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject) -> *mut AnyObject,
                objc2::runtime::Imp,
            >(application_dock_menu);
            let success = objc_ffi::class_addMethod(
                cls_ptr,
                sel!(applicationDockMenu:),
                dock_menu_imp,
                b"@24@0:8@16\0".as_ptr() as *const c_char,
            );
            println!("Dock menu injected into TaoAppDelegateParent: {}", success.as_bool());

            // Inject application:openFile: — handles Finder file opens via CFBundleDocumentTypes.
            // Type encoding: B32@0:8@16@24 — bool return, (self id, SEL, NSApplication* id, NSString* id)
            let open_file_imp: objc2::runtime::Imp = std::mem::transmute::<
                unsafe extern "C-unwind" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject, *mut AnyObject) -> bool,
                objc2::runtime::Imp,
            >(application_open_file);
            let open_file_success = objc_ffi::class_addMethod(
                cls_ptr,
                sel!(application:openFile:),
                open_file_imp,
                b"B32@0:8@16@24\0".as_ptr() as *const c_char,
            );
            println!("application:openFile: injected into TaoAppDelegateParent: {}", open_file_success.as_bool());
        } else {
            eprintln!("TaoAppDelegateParent class not found; dock menu skipped");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(FileWatchers::default())
        .manage(EmptyWindows::default())
        .manage(PendingFiles::default())
        .manage(ReadyWindows::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("Single instance callback - argv: {:?}, cwd: {:?}", argv, cwd);

            // Look for file arguments in the new instance
            // Create a NEW window for each file instead of reusing existing
            for arg in argv.iter().skip(1) {
                let path = std::path::Path::new(arg);
                if path.exists() && (arg.ends_with(".md") || arg.ends_with(".markdown") || arg.ends_with(".txt")) {
                    println!("Found file to open from second instance: {}", arg);

                    // Read the file and create a new window
                    if let Ok(content) = fs::read_to_string(arg) {
                        match create_document_window(app, Some(arg.clone()), Some(content)) {
                            Ok(_) => println!("Created new window for file: {}", arg),
                            Err(e) => eprintln!("Failed to create window for file {}: {}", arg, e),
                        }
                    }
                    // Continue to create windows for all file arguments, not just the first
                }
            }
        }))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Process command line arguments for first instance startup
            let args: Vec<String> = std::env::args().collect();
            println!("App setup - found {} arguments:", args.len());
            for (i, arg) in args.iter().enumerate() {
                println!("  Setup Arg {}: {}", i, arg);
            }

            // Check if any file arguments were passed
            let mut file_to_open: Option<(String, String)> = None;
            for arg in args.iter().skip(1) {
                let path = std::path::Path::new(arg);
                let is_markdown = arg.ends_with(".md") || arg.ends_with(".markdown") || arg.ends_with(".txt");

                if path.exists() && is_markdown {
                    println!("Found file to open from first instance: {}", arg);

                    match fs::read_to_string(arg) {
                        Ok(content) => {
                            file_to_open = Some((arg.clone(), content));
                            break;
                        }
                        Err(e) => {
                            println!("Error reading file {}: {}", arg, e);
                        }
                    }
                }
            }

            // Create menu
            let app_menu = SubmenuBuilder::new(app, "Mark-us-Down")
                .item(&MenuItemBuilder::new("About Mark-us-Down").id("about").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Quit Mark-us-Down").id("quit").accelerator("CmdOrCtrl+Q").build(app)?)
                .build()?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&MenuItemBuilder::new("New Window").id("new_window").accelerator("CmdOrCtrl+Shift+N").build(app)?)
                .item(&MenuItemBuilder::new("New Tab").id("new_tab").accelerator("CmdOrCtrl+T").build(app)?)
                .item(&MenuItemBuilder::new("New").id("new").accelerator("CmdOrCtrl+N").build(app)?)
                .item(&MenuItemBuilder::new("Open...").id("open").accelerator("CmdOrCtrl+O").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Save").id("save").accelerator("CmdOrCtrl+S").build(app)?)
                .item(&MenuItemBuilder::new("Save As...").id("save_as").accelerator("CmdOrCtrl+Shift+S").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Print...").id("print").accelerator("CmdOrCtrl+P").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Close").id("close").accelerator("CmdOrCtrl+W").build(app)?)
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&MenuItemBuilder::new("Undo").id("undo").accelerator("CmdOrCtrl+Z").build(app)?)
                .item(&MenuItemBuilder::new("Redo").id("redo").accelerator("CmdOrCtrl+Shift+Z").build(app)?)
                .separator()
                .item(&PredefinedMenuItem::cut(app, None)?)
                .item(&PredefinedMenuItem::copy(app, None)?)
                .item(&PredefinedMenuItem::paste(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::select_all(app, None)?)
                .build()?;

            let view_menu_builder = SubmenuBuilder::new(app, "View")
                .item(&MenuItemBuilder::new("Switch to Dark Mode").id("theme_toggle").accelerator("CmdOrCtrl+Shift+L").build(app)?)
                .item(&MenuItemBuilder::new("Toggle Reading Mode").id("reading_mode").accelerator("CmdOrCtrl+E").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Zoom In").id("zoom_in").accelerator("CmdOrCtrl+Plus").build(app)?)
                .item(&MenuItemBuilder::new("Zoom Out").id("zoom_out").accelerator("CmdOrCtrl+-").build(app)?)
                .item(&MenuItemBuilder::new("Reset Zoom").id("reset_zoom").accelerator("CmdOrCtrl+0").build(app)?);

            #[cfg(debug_assertions)]
            let view_menu_builder = view_menu_builder
                .separator()
                .item(&MenuItemBuilder::new("Debug Info").id("debug_info").build(app)?);

            let view_menu = view_menu_builder.build()?;

            let window_menu = build_window_menu(app)?;

            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .item(&window_menu)
                .build()?;

            app.set_menu(menu)?;

            // Set up the macOS dock right-click menu
            #[cfg(target_os = "macos")]
            setup_dock_menu(app.handle());

            // If launched with a file argument, create that window now on the main thread.
            // For normal (no-file) launches, defer empty window creation to RunEvent::Ready
            // so that application:openFile: (macOS Finder double-click) has a chance to
            // fire first — preventing a stale empty window from being created alongside
            // the file window that application:openFile: opens.
            if let Some((file_path, content)) = file_to_open {
                // Launched from the command line with a file argument
                if let Err(e) = create_document_window(app.handle(), Some(file_path), Some(content)) {
                    eprintln!("Failed to create window for file: {}", e);
                }
                FILE_OPEN_HANDLED.store(true, Ordering::SeqCst);
            }
            // else: no window created here; RunEvent::Ready creates the empty window
            // if no file open has occurred by then.

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            new_file,
            create_new_window,
            save_file_dialog,
            save_file,
            read_file,
            read_binary_file,
            open_file_dialog,
            update_theme_menu,
            debug_args,
            start_file_watcher,
            stop_file_watcher,
            set_window_empty,
            window_ready
        ])
        .on_menu_event(handle_menu_event)
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { .. } => {
                let window_label = window.label().to_string();
                println!("Window close requested: {}", window_label);

                let app_handle = window.app_handle();

                // Clean up file watchers for this window
                cleanup_window_watchers(&app_handle, &window_label);

                // Remove from empty windows tracking
                {
                    let empty_windows: tauri::State<EmptyWindows> = app_handle.state::<EmptyWindows>();
                    let mut empty_set = empty_windows.inner().lock().unwrap();
                    empty_set.remove(&window_label);
                }

                // Remove from ready/pending tracking
                {
                    let ready_windows: tauri::State<ReadyWindows> = app_handle.state::<ReadyWindows>();
                    ready_windows.inner().0.lock().unwrap().remove(&window_label);
                }
                {
                    let pending_files: tauri::State<PendingFiles> = app_handle.state::<PendingFiles>();
                    pending_files.inner().0.lock().unwrap().remove(&window_label);
                }

                // Count remaining windows
                let windows = app_handle.webview_windows();
                let window_count = windows.len();
                println!("Remaining windows (including this one): {}", window_count);

                // On macOS, only exit if this is the last window
                // On other platforms, exit when all windows are closed
                #[cfg(target_os = "macos")]
                {
                    if window_count <= 1 {
                        // This is the last window - the app will stay in the dock
                        // macOS apps typically don't exit when all windows close
                        println!("Last window closing on macOS - app will stay running");
                    }
                    // Let the window close naturally
                }

                #[cfg(not(target_os = "macos"))]
                {
                    if window_count <= 1 {
                        println!("Last window closing - exiting app");
                        app_handle.exit(0);
                    }
                    // Let the window close naturally
                }
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                println!("Drag drop event received with {} files in window: {}", paths.len(), window.label());
                let window_label = window.label().to_string();
                // Handle dropped files - open in THIS window (not create new ones)
                for path in paths {
                    println!("Processing dropped file: {:?}", path);
                    if let Some(extension) = path.extension() {
                        if extension == "md" || extension == "markdown" || extension == "txt" {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    println!("Opening dropped file in window: {}", window_label);
                                    match window.emit_to(&window_label, "file-opened", (path.to_string_lossy().to_string(), content)) {
                                        Ok(_) => println!("Successfully emitted file-opened event to {}", window_label),
                                        Err(e) => println!("Failed to emit file-opened event: {}", e),
                                    }
                                    break; // Only open the first markdown/text file
                                }
                                Err(e) => {
                                    eprintln!("Error reading file {:?}: {}", path, e);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                // macOS-specific file opening via URL scheme (CFBundleURLTypes).
                // Note: Finder double-click / "Open With" via CFBundleDocumentTypes goes through
                // application:openFile: (injected above), NOT here.  This handler catches
                // any URL-scheme opens that tao routes through application:openURLs:.
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Opened { urls } => {
                    for url in urls {
                        let path = url.to_file_path().unwrap_or_else(|_| std::path::PathBuf::from(url.as_str()));
                        let path_str = path.to_string_lossy().to_string();
                        if path.exists() && (path_str.ends_with(".md") || path_str.ends_with(".markdown") || path_str.ends_with(".txt")) {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    handle_file_open(app_handle, path_str, content);
                                    FILE_OPEN_HANDLED.store(true, Ordering::SeqCst);
                                }
                                Err(e) => eprintln!("Error reading opened file {}: {}", path_str, e),
                            }
                        }
                    }
                }
                // Handle reopen event (clicking dock icon when no windows are open)
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                    println!("Reopen event - has_visible_windows: {}", has_visible_windows);
                    if !has_visible_windows {
                        // Create a new empty window when clicking dock icon with no windows
                        match create_document_window(app_handle, None, None) {
                            Ok(_) => println!("Created new window on reopen"),
                            Err(e) => eprintln!("Failed to create window on reopen: {}", e),
                        }
                    }
                }
                // Primary path for normal (no-file) launches: setup() defers empty
                // window creation here so that application:openFile: (Finder cold-start)
                // can fire first. If a file was already opened, this is a no-op.
                tauri::RunEvent::Ready => {
                    let windows = app_handle.webview_windows();
                    if windows.is_empty() && !FILE_OPEN_HANDLED.load(Ordering::SeqCst) {
                        println!("RunEvent::Ready: no window found, creating fallback window");
                        if let Err(e) = create_document_window(app_handle, None, None) {
                            eprintln!("Failed to create fallback window: {}", e);
                        }
                    }
                }
                _ => {}
            }
        });
}

fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    println!("Menu event received: {}", event.id().as_ref());

    // Get the target window (focused or first available)
    let target_window = get_target_window(app);

    match event.id().as_ref() {
        "new_window" => {
            // Create a new empty window
            println!("Creating new window from menu");
            match create_document_window_with_placement(app, None, None, Placement::Window) {
                Ok(_) => println!("Created new window from menu"),
                Err(e) => eprintln!("Failed to create new window: {}", e),
            }
        }
        "new_tab" => {
            println!("Creating new tab from menu");
            if let Err(e) = create_document_window(app, None, None) {
                eprintln!("Failed to create new tab: {}", e);
            }
        }
        "new" => {
            // Handle new file in current window
            println!("Handling new file menu");
            if let Some(window) = target_window {
                match window.emit_to(window.label(), "menu-new-file", ()) {
                    Ok(_) => println!("Successfully emitted menu-new-file event"),
                    Err(e) => println!("Failed to emit menu-new-file event: {}", e),
                }
            } else {
                // No windows open, create a new one
                let _ = create_document_window(app, None, None);
            }
        }
        "open" => {
            println!("Handling open file menu");
            if let Some(window) = target_window {
                let app_handle = app.clone();
                let window_clone = window.clone();
                tauri::async_runtime::spawn(async move {
                    match open_file_dialog(window_clone, app_handle).await {
                        Ok(_) => println!("File dialog opened successfully"),
                        Err(e) => println!("Failed to open file dialog: {}", e),
                    }
                });
            }
        }
        "save" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-save-file", ());
            }
        }
        "save_as" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-save-as-file", ());
            }
        }
        "print" => {
            if let Some(window) = target_window {
                match window.print() {
                    Ok(_) => println!("Print dialog opened"),
                    Err(e) => eprintln!("Failed to open print dialog: {}", e),
                }
            }
        }
        "close" => {
            if let Some(window) = target_window {
                let _ = window.close();
            }
        }
        "quit" => {
            app.exit(0);
        }
        "undo" => {
            println!("Undo requested");
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-undo", ());
            }
        }
        "redo" => {
            println!("Redo requested");
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-redo", ());
            }
        }
        "theme_toggle" => {
            println!("Menu theme_toggle clicked");
            if let Some(window) = target_window {
                match window.emit_to(window.label(), "menu-toggle-theme", ()) {
                    Ok(_) => println!("Successfully emitted menu-toggle-theme event"),
                    Err(e) => println!("Failed to emit menu-toggle-theme event: {}", e),
                }
            }
        }
        "zoom_in" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-zoom-in", ());
            }
        }
        "zoom_out" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-zoom-out", ());
            }
        }
        "reset_zoom" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-reset-zoom", ());
            }
        }
        "reading_mode" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-toggle-reading-mode", ());
            }
        }
        "about" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-about", ());
            }
        }
        #[cfg(debug_assertions)]
        "debug_info" => {
            if let Some(window) = target_window {
                let _ = window.emit_to(window.label(), "menu-debug-info", ());
            }
        }
        _ => {}
    }
}
