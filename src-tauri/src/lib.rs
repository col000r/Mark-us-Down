use tauri::{Manager, menu::*, Emitter, WindowEvent, WebviewUrl};
use tauri::webview::WebviewWindowBuilder;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;

// Counter for generating unique window labels
static WINDOW_COUNTER: AtomicU64 = AtomicU64::new(0);

// Flag to track if a window was created from a file open event (macOS)
static FILE_OPEN_HANDLED: AtomicBool = AtomicBool::new(false);

// File watcher state - keyed by (window_label, file_path) to support per-window watchers
type FileWatchers = Arc<Mutex<HashMap<String, RecommendedWatcher>>>;

/// Generate a unique window label
fn generate_window_label() -> String {
    let count = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("doc-{}", count)
}

/// Create a new document window, optionally with a file to open
fn create_document_window(
    app_handle: &tauri::AppHandle,
    file_path: Option<String>,
    content: Option<String>,
) -> Result<tauri::WebviewWindow, String> {
    let label = generate_window_label();
    println!("Creating new document window with label: {}", label);

    let window = WebviewWindowBuilder::new(app_handle, &label, WebviewUrl::App("index.html".into()))
        .title("Mark-us-Down")
        .inner_size(1200.0, 800.0)
        .min_inner_size(600.0, 400.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;

    // If we have a file to open, emit the event after a short delay to let frontend initialize
    if let (Some(path), Some(file_content)) = (file_path, content) {
        let window_clone = window.clone();
        let window_label = window.label().to_string();
        tauri::async_runtime::spawn(async move {
            // Give the frontend time to initialize
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            match window_clone.emit_to(&window_label, "file-opened", (&path, &file_content)) {
                Ok(_) => println!("Successfully emitted file-opened event to new window for: {}", path),
                Err(e) => eprintln!("Failed to emit file-opened event: {}", e),
            }
        });
    }

    Ok(window)
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
        .item(&MenuItemBuilder::new("New").id("new").accelerator("CmdOrCtrl+N").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Open...").id("open").accelerator("CmdOrCtrl+O").build(&app_handle)?)
        .separator()
        .item(&MenuItemBuilder::new("Save").id("save").accelerator("CmdOrCtrl+S").build(&app_handle)?)
        .item(&MenuItemBuilder::new("Save As...").id("save_as").accelerator("CmdOrCtrl+Shift+S").build(&app_handle)?)
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
        .item(&MenuItemBuilder::new(theme_text).id("theme_toggle").accelerator("CmdOrCtrl+T").build(&app_handle)?)
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

    let menu = MenuBuilder::new(&app_handle)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(FileWatchers::default())
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
                .item(&MenuItemBuilder::new("New").id("new").accelerator("CmdOrCtrl+N").build(app)?)
                .item(&MenuItemBuilder::new("Open...").id("open").accelerator("CmdOrCtrl+O").build(app)?)
                .separator()
                .item(&MenuItemBuilder::new("Save").id("save").accelerator("CmdOrCtrl+S").build(app)?)
                .item(&MenuItemBuilder::new("Save As...").id("save_as").accelerator("CmdOrCtrl+Shift+S").build(app)?)
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
                .item(&MenuItemBuilder::new("Switch to Dark Mode").id("theme_toggle").accelerator("CmdOrCtrl+T").build(app)?)
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

            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .build()?;

            app.set_menu(menu)?;

            // If a file was passed via command line, emit it to the main window after a delay
            if let Some((file_path, content)) = file_to_open {
                let window = app.get_webview_window("main").unwrap();
                let window_clone = window.clone();
                let window_label = window.label().to_string();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    match window_clone.emit_to(&window_label, "file-opened", (file_path.clone(), content)) {
                        Ok(_) => println!("Successfully emitted file-opened event for: {}", file_path),
                        Err(e) => eprintln!("Failed to emit file-opened event: {}", e),
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            new_file,
            create_new_window,
            save_file_dialog,
            save_file,
            read_file,
            open_file_dialog,
            update_theme_menu,
            debug_args,
            start_file_watcher,
            stop_file_watcher
        ])
        .on_menu_event(handle_menu_event)
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { .. } => {
                let window_label = window.label().to_string();
                println!("Window close requested: {}", window_label);

                let app_handle = window.app_handle();

                // Clean up file watchers for this window
                cleanup_window_watchers(&app_handle, &window_label);

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
                // macOS-specific file opening via "Open With" or double-click
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Opened { urls } => {
                    // Handle file opening from macOS "Open With" or double-click
                    // Note: On cold start, this fires BEFORE any windows exist (windows array is empty in config)
                    let mut first_file = true;

                    for url in urls {
                        let path = url.to_file_path().unwrap_or_else(|_| std::path::PathBuf::from(url.as_str()));
                        let path_str = path.to_string_lossy().to_string();

                        if path.exists() && (path_str.ends_with(".md") || path_str.ends_with(".markdown") || path_str.ends_with(".txt")) {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    // For the first file, try to reuse any existing window
                                    if first_file {
                                        let reuse_window = app_handle.get_webview_window("main")
                                            .or_else(|| app_handle.webview_windows().into_values().next());

                                        if let Some(existing_window) = reuse_window {
                                            // Reuse existing window for the first file
                                            let window_label = existing_window.label().to_string();
                                            let window_clone = existing_window.clone();
                                            let path_clone = path_str.clone();
                                            tauri::async_runtime::spawn(async move {
                                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                                let _ = window_clone.emit_to(&window_label, "file-opened", (&path_clone, &content));
                                            });
                                            FILE_OPEN_HANDLED.store(true, Ordering::SeqCst);
                                            first_file = false;
                                            continue;
                                        }
                                    }

                                    // Create a new window for the file
                                    match create_document_window(app_handle, Some(path_str.clone()), Some(content)) {
                                        Ok(_) => {
                                            FILE_OPEN_HANDLED.store(true, Ordering::SeqCst);
                                        },
                                        Err(e) => eprintln!("Failed to create window for file {}: {}", path_str, e),
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading opened file {}: {}", path_str, e);
                                }
                            }
                            first_file = false;
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
                // When the app is ready, check if we need to create a default window
                // This handles normal launch (not via file double-click)
                tauri::RunEvent::Ready => {
                    let app_handle_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Wait a bit to allow RunEvent::Opened to fire first (if launching via file)
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                        // Check if any window was created
                        let windows = app_handle_clone.webview_windows();
                        if windows.is_empty() && !FILE_OPEN_HANDLED.load(Ordering::SeqCst) {
                            // No windows and no file was opened - create a default window
                            match create_document_window(&app_handle_clone, None, None) {
                                Ok(_) => println!("Created default window on startup"),
                                Err(e) => eprintln!("Failed to create default window: {}", e),
                            }
                        }
                    });
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
            match create_document_window(app, None, None) {
                Ok(_) => println!("Created new window from menu"),
                Err(e) => eprintln!("Failed to create new window: {}", e),
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
