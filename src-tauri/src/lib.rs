use tauri::{Manager, menu::*, Emitter, WindowEvent, RunEvent};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// Store pending file to open when window becomes available
static PENDING_FILE: Mutex<Option<(String, String)>> = Mutex::new(None);

// Tauri commands for file operations
#[tauri::command]
async fn new_file(app_handle: tauri::AppHandle) -> Result<(), String> {
  // Reset the file state
  let window = app_handle.get_webview_window("main").unwrap();
  window.emit("file-new", ()).map_err(|e| e.to_string())?;
  Ok(())
}



#[tauri::command]
async fn save_file_dialog(app_handle: tauri::AppHandle, content: String) -> Result<Option<String>, String> {
  use tauri_plugin_dialog::{DialogExt};
  
  let dialog = app_handle.dialog().file()
    .add_filter("Markdown files", &["md"])
    .set_title("Save Markdown File")
    .set_file_name("untitled.md");
  
  dialog.save_file(move |path| {
    if let Some(path) = path {
      let path_str = path.to_string();
      let path_buf = PathBuf::from(path_str);
      match fs::write(&path_buf, &content) {
        Ok(_) => {
          if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("file-saved", path_buf.to_string_lossy().to_string());
          }
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
async fn save_file(app_handle: tauri::AppHandle, path: String, content: String) -> Result<(), String> {
  fs::write(&path, content).map_err(|e| e.to_string())?;
  let window = app_handle.get_webview_window("main").unwrap();
  window.emit("file-saved", path).map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
async fn read_file(app_handle: tauri::AppHandle, path: String) -> Result<String, String> {
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  let window = app_handle.get_webview_window("main").unwrap();
  window.emit("file-opened", (path, content.clone())).map_err(|e| e.to_string())?;
  Ok(content)
}


#[tauri::command]
async fn update_theme_menu(app_handle: tauri::AppHandle, is_dark: bool) -> Result<(), tauri::Error> {
  println!("🔄 update_theme_menu called with is_dark: {}", is_dark);
  println!("📝 Current theme state: {}", if is_dark { "DARK" } else { "LIGHT" });
  
  // Rebuild the entire menu with updated theme text
  let theme_text = if is_dark {
    "Switch to Light Mode"
  } else {
    "Switch to Dark Mode"
  };
  
  println!("🏷️  Menu text will be: '{}'", theme_text);
  
  // Rebuild the menu
  let app_menu = SubmenuBuilder::new(&app_handle, "Mark-us-Down")
    .item(&MenuItemBuilder::new("About Mark-us-Down").id("about").build(&app_handle)?)
    .separator()
    .item(&MenuItemBuilder::new("Quit Mark-us-Down").id("quit").accelerator("CmdOrCtrl+Q").build(&app_handle)?)
    .build()?;

  let file_menu = SubmenuBuilder::new(&app_handle, "File")
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

  let view_menu = SubmenuBuilder::new(&app_handle, "View")
    .item(&MenuItemBuilder::new(theme_text).id("theme_toggle").accelerator("CmdOrCtrl+T").build(&app_handle)?)
    .separator()
    .item(&MenuItemBuilder::new("Zoom In").id("zoom_in").accelerator("CmdOrCtrl+Plus").build(&app_handle)?)
    .item(&MenuItemBuilder::new("Zoom Out").id("zoom_out").accelerator("CmdOrCtrl+-").build(&app_handle)?)
    .item(&MenuItemBuilder::new("Reset Zoom").id("reset_zoom").accelerator("CmdOrCtrl+0").build(&app_handle)?)
    .separator()
    .item(&MenuItemBuilder::new("Debug Info").id("debug_info").build(&app_handle)?)
    .build()?;

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
async fn open_file_dialog(app_handle: tauri::AppHandle) -> Result<(), String> {
  use tauri_plugin_dialog::{DialogExt};
  
  let dialog = app_handle.dialog().file()
    .add_filter("Markdown files", &["md", "markdown", "txt"])
    .set_title("Open Markdown File");
  
  dialog.pick_file(move |path| {
    if let Some(path) = path {
      let path_buf = std::path::PathBuf::from(path.as_path().unwrap());
      let path_str = path_buf.to_string_lossy().to_string();
      match fs::read_to_string(&path_buf) {
        Ok(content) => {
          if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("file-opened", (path_str, content));
          }
        }
        Err(e) => {
          eprintln!("Error reading file: {}", e);
        }
      }
    }
  });
  
  Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Initial debug log
  let debug_info = "Application started - waiting for RunEvent::Opened events for file associations.\n";
  if let Some(home_dir) = std::env::var_os("HOME") {
    let debug_path = std::path::Path::new(&home_dir).join("mark_us_down_debug.log");
    let _ = fs::write(&debug_path, debug_info);
  }
  
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
      println!("Single instance callback - argv: {:?}, cwd: {:?}", argv, cwd);
      
      // Look for file arguments in the new instance
      for arg in argv.iter().skip(1) {
        if std::path::Path::new(arg).exists() && (arg.ends_with(".md") || arg.ends_with(".markdown") || arg.ends_with(".txt")) {
          println!("Found file to open from second instance: {}", arg);
          
          // Read the file and emit to the existing window
          if let Ok(content) = fs::read_to_string(arg) {
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.emit("file-opened", (arg.clone(), content));
              println!("Emitted file-opened event for: {}", arg);
            }
          }
          break;
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

      // Create menu
      // On macOS, the app menu contains About and Quit
      let app_menu = SubmenuBuilder::new(app, "Mark-us-Down")
        .item(&MenuItemBuilder::new("About Mark-us-Down").id("about").build(app)?)
        .separator()
        .item(&MenuItemBuilder::new("Quit Mark-us-Down").id("quit").accelerator("CmdOrCtrl+Q").build(app)?)
        .build()?;

      let file_menu = SubmenuBuilder::new(app, "File")
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

      let view_menu = SubmenuBuilder::new(app, "View")
        .item(&MenuItemBuilder::new("Switch to Dark Mode").id("theme_toggle").accelerator("CmdOrCtrl+T").build(app)?)
        .separator()
        .item(&MenuItemBuilder::new("Zoom In").id("zoom_in").accelerator("CmdOrCtrl+Plus").build(app)?)
        .item(&MenuItemBuilder::new("Zoom Out").id("zoom_out").accelerator("CmdOrCtrl+-").build(app)?)
        .item(&MenuItemBuilder::new("Reset Zoom").id("reset_zoom").accelerator("CmdOrCtrl+0").build(app)?)
        .separator()
        .item(&MenuItemBuilder::new("Debug Info").id("debug_info").build(app)?)
        .build()?;

      let menu = MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()?;

      app.set_menu(menu)?;

      // Check for any pending file that was opened before the window was ready
      let window = app.get_webview_window("main").unwrap();
      if let Ok(mut pending) = PENDING_FILE.lock() {
        if let Some((file_path, content)) = pending.take() {
          println!("🔄 Processing pending file after window creation: {}", file_path);
          
          // Give the frontend a moment to initialize
          let window_clone = window.clone();
          tauri::async_runtime::spawn(async move {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            match window_clone.emit("file-opened", (file_path.clone(), content)) {
              Ok(_) => println!("✅ Successfully emitted file-opened event for pending file: {}", file_path),
              Err(e) => eprintln!("❌ Failed to emit file-opened event for pending file: {}", e),
            }
          });
        }
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      new_file,
      save_file_dialog,
      save_file,
      read_file,
      open_file_dialog,
      update_theme_menu,
      debug_args
    ])
    .on_menu_event(handle_menu_event)
    .on_window_event(|window, event| match event {
      WindowEvent::CloseRequested { .. } => {
        // Handle window close with proper cleanup
        window.close().unwrap();
      }
      WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
        println!("Drag drop event received with {} files", paths.len());
        // Handle dropped files
        for path in paths {
          println!("Processing dropped file: {:?}", path);
          if let Some(extension) = path.extension() {
            println!("File extension: {:?}", extension);
            if extension == "md" || extension == "markdown" || extension == "txt" {
              // Read the first markdown/text file and emit event
              match fs::read_to_string(&path) {
                Ok(content) => {
                  println!("Successfully read file, emitting file-opened event");
                  match window.emit("file-opened", (path.to_string_lossy().to_string(), content)) {
                    Ok(_) => println!("Successfully emitted file-opened event"),
                    Err(e) => println!("Failed to emit file-opened event: {}", e),
                  }
                  break; // Only open the first markdown/text file
                }
                Err(e) => {
                  eprintln!("Error reading file {:?}: {}", path, e);
                  continue;
                }
              }
            } else {
              println!("Skipping file with unsupported extension: {:?}", extension);
            }
          } else {
            println!("File has no extension: {:?}", path);
          }
        }
      }
      _ => {}
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app_handle, event| {
      match event {
        RunEvent::Opened { urls } => {
          println!("🔥 RunEvent::Opened received with {} URLs", urls.len());
          
          // Log to file for debugging
          let mut debug_info = format!("RunEvent::Opened received with {} URLs:\n", urls.len());
          for (i, url) in urls.iter().enumerate() {
            let url_line = format!("  URL {}: {}\n", i, url);
            debug_info.push_str(&url_line);
            println!("  URL {}: {}", i, url);
          }
          
          // Write to debug file
          if let Some(home_dir) = std::env::var_os("HOME") {
            let debug_path = std::path::Path::new(&home_dir).join("mark_us_down_debug.log");
            let _ = fs::write(&debug_path, &debug_info);
          }
          
          // Process the first markdown file
          for url in urls {
            let url_str = url.to_string();
            debug_info.push_str(&format!("Processing URL: {}\n", url_str));
            
            // Convert file:// URL to file path
            let file_path = if url_str.starts_with("file://") {
              url_str.strip_prefix("file://").unwrap_or(&url_str)
            } else {
              &url_str
            };
            
            println!("🔍 Processing file path: {}", file_path);
            debug_info.push_str(&format!("Converted to file path: {}\n", file_path));
            
            let path = std::path::Path::new(file_path);
            let exists = path.exists();
            let is_markdown = file_path.ends_with(".md") || file_path.ends_with(".markdown") || file_path.ends_with(".txt");
            
            println!("   File exists: {}", exists);
            println!("   Is markdown: {}", is_markdown);
            debug_info.push_str(&format!("File exists: {}\n", exists));
            debug_info.push_str(&format!("Is markdown: {}\n", is_markdown));
            
            if exists && is_markdown {
              println!("✅ Found valid markdown file: {}", file_path);
              debug_info.push_str(&format!("✅ Valid markdown file found\n"));
              
              match fs::read_to_string(path) {
                Ok(content) => {
                  println!("📖 Successfully read file content ({} chars)", content.len());
                  debug_info.push_str(&format!("Successfully read {} characters\n", content.len()));
                  
                  // Check if window exists
                  let windows = app_handle.webview_windows();
                  println!("Available windows: {:?}", windows.keys().collect::<Vec<_>>());
                  debug_info.push_str(&format!("Available windows: {:?}\n", windows.keys().collect::<Vec<_>>()));
                  
                  // Try multiple ways to get the window
                  let window = app_handle.get_webview_window("main")
                    .or_else(|| windows.values().next().cloned());
                  
                  if let Some(window) = window {
                    println!("📤 Found window, emitting file-opened event...");
                    debug_info.push_str("Found window, emitting event...\n");
                    
                    // Add a small delay to ensure the frontend is ready
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    match window.emit("file-opened", (file_path.to_string(), content)) {
                      Ok(_) => {
                        println!("✅ Successfully emitted file-opened event for: {}", file_path);
                        debug_info.push_str(&format!("✅ Successfully emitted event for: {}\n", file_path));
                      },
                      Err(e) => {
                        eprintln!("❌ Failed to emit file-opened event: {}", e);
                        debug_info.push_str(&format!("❌ Failed to emit event: {}\n", e));
                      }
                    }
                  } else {
                    // No window available yet - store the file to open later
                    println!("📦 No window available, storing file for later: {}", file_path);
                    debug_info.push_str(&format!("📦 Storing file for later: {}\n", file_path));
                    
                    if let Ok(mut pending) = PENDING_FILE.lock() {
                      *pending = Some((file_path.to_string(), content));
                      println!("✅ File stored successfully");
                      debug_info.push_str("✅ File stored successfully\n");
                    } else {
                      eprintln!("❌ Failed to store pending file");
                      debug_info.push_str("❌ Failed to store pending file\n");
                    }
                  }
                }
                Err(e) => {
                  eprintln!("❌ Error reading file '{}': {}", file_path, e);
                  debug_info.push_str(&format!("❌ Error reading file: {}\n", e));
                }
              }
              break; // Only process the first valid file
            } else {
              println!("⏭️ Skipping file: {} (exists: {}, is_markdown: {})", file_path, exists, is_markdown);
              debug_info.push_str(&format!("⏭️ Skipping file (exists: {}, is_markdown: {})\n", exists, is_markdown));
            }
          }
          
          // Update debug file with all the processing info
          if let Some(home_dir) = std::env::var_os("HOME") {
            let debug_path = std::path::Path::new(&home_dir).join("mark_us_down_debug.log");
            let _ = fs::write(&debug_path, &debug_info);
          }
        }
        _ => {}
      }
    });
}

fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
  println!("Menu event received: {}", event.id().as_ref());
  match event.id().as_ref() {
    "new" => {
      // Handle new file
      println!("Handling new file menu");
      if let Some(window) = app.get_webview_window("main") {
        match window.emit("menu-new-file", ()) {
          Ok(_) => println!("Successfully emitted menu-new-file event"),
          Err(e) => println!("Failed to emit menu-new-file event: {}", e),
        }
      } else {
        println!("Could not get main window for new file");
      }
    }
    "open" => {
      println!("Handling open file menu");
      // Call the command directly instead of emitting an event
      let app_handle = app.clone();
      tauri::async_runtime::spawn(async move {
        match open_file_dialog(app_handle).await {
          Ok(_) => println!("File dialog opened successfully"),
          Err(e) => println!("Failed to open file dialog: {}", e),
        }
      });
    }
    "save" => {
      // Handle save file
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-save-file", ()).unwrap();
      }
    }
    "save_as" => {
      // Handle save as file
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-save-as-file", ()).unwrap();
      }
    }
    "close" => {
      // Handle close window
      if let Some(window) = app.get_webview_window("main") {
        window.close().unwrap();
      }
    }
    "quit" => {
      // Handle quit application
      app.exit(0);
    }
    "undo" => {
      // Handle undo
      println!("Undo requested");
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-undo", ()).unwrap();
      }
    }
    "redo" => {
      // Handle redo
      println!("Redo requested");
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-redo", ()).unwrap();
      }
    }
    "theme_toggle" => {
      // Handle theme toggle
      println!("🔔 Menu theme_toggle clicked");
      if let Some(window) = app.get_webview_window("main") {
        match window.emit("menu-toggle-theme", ()) {
          Ok(_) => println!("✅ Successfully emitted menu-toggle-theme event"),
          Err(e) => println!("❌ Failed to emit menu-toggle-theme event: {}", e),
        }
      }
    }
    "zoom_in" => {
      // Handle zoom in
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-zoom-in", ()).unwrap();
      }
    }
    "zoom_out" => {
      // Handle zoom out
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-zoom-out", ()).unwrap();
      }
    }
    "reset_zoom" => {
      // Handle reset zoom
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-reset-zoom", ()).unwrap();
      }
    }
    "about" => {
      // Handle about dialog
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-about", ()).unwrap();
      }
    }
    "debug_info" => {
      // Handle debug info toggle
      if let Some(window) = app.get_webview_window("main") {
        window.emit("menu-debug-info", ()).unwrap();
      }
    }
    _ => {}
  }
}
