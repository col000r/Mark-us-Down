import { useEffect, useState, useRef } from 'react'
import * as monaco from 'monaco-editor'
import './App.css'
import './styles/highlight.css'
import { SplitView, SourceEditor, PreviewPane } from './components'
import { scrollSyncService } from './services/scrollSync'

function App() {

  const [content, setContent] = useState('')
  const [currentFile, setCurrentFile] = useState<string | null>(null)
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false)
  const [isDarkTheme, setIsDarkTheme] = useState(() => {
    const saved = localStorage.getItem('markdown-editor-theme')
    return saved === 'dark'
  })
  const [isDragOver, setIsDragOver] = useState(false)
  const [showAbout, setShowAbout] = useState(false)
  const [documentTitle, setDocumentTitle] = useState<string | null>(null)
  const [isWeb, setIsWeb] = useState(false) // Start as false (hide buttons), set true only if web
  const [debugInfo, setDebugInfo] = useState<string>('')
  const isTauri = !isWeb // Derived value for backward compatibility
  
  // Use a ref to track the current theme state to avoid stale closures
  const isDarkThemeRef = useRef(isDarkTheme)
  // Use a ref to track the Monaco editor instance for clipboard operations
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  
  // Update the ref whenever the state changes
  useEffect(() => {
    isDarkThemeRef.current = isDarkTheme
  }, [isDarkTheme])

  useEffect(() => {
    console.log('Markdown Editor initialized');
    console.info('Frontend loaded successfully');
    console.log('Window location protocol:', window.location.protocol);
    console.log('Has __TAURI__:', '__TAURI__' in window);
    console.log('Has __TAURI_INTERNALS__:', '__TAURI_INTERNALS__' in window);
    console.log('Window object keys:', Object.keys(window).filter(k => k.includes('TAURI')));
    
    let unlisten: (() => void) | undefined;
    
    // Dynamic Tauri/Web detection
    const detectEnvironment = async () => {
      try {
        // Try to import Tauri API - if this works, we're in Tauri
        await import('@tauri-apps/api/core');
        setIsWeb(false); // Tauri mode - keep buttons hidden
        console.log('Successfully detected Tauri environment');
        return true; // is Tauri
      } catch (error) {
        setIsWeb(true); // Web mode - show buttons
        console.log('Not in Tauri environment:', error);
        return false; // is Web
      }
    };
    
    // Check if we're in Tauri and set up listeners
    const initializeApp = async () => {
      const inTauri = await detectEnvironment();
      console.log('Environment detected - inTauri:', inTauri);
      
      if (import.meta.env.DEV) {
        console.log('Running in development mode');
      }

      if (!inTauri) {
        console.log('Not running in Tauri, skipping event setup');
        return;
      }
      
      console.log('Running in Tauri mode, setting up listeners...');

      const setupListeners = async () => {
      try {
        console.log('Setting up Tauri event listeners...');
        const { listen } = await import('@tauri-apps/api/event');
        
        // Set up the file-opened listener immediately first
        const fileOpenedListener = await listen<[string, string]>('file-opened', (event) => {
          console.log('File opened event received:', event);
          console.log('Event payload:', event.payload);
          console.log('Payload type:', typeof event.payload);
          console.log('Is array:', Array.isArray(event.payload));
          if (Array.isArray(event.payload) && event.payload.length === 2) {
            const [filePath, fileContent] = event.payload;
            console.log('Setting file:', filePath, 'with content length:', fileContent.length);
            setCurrentFile(filePath);
            setContent(fileContent);
            setHasUnsavedChanges(false);
            setIsDragOver(false); // Clear drag state when file loads successfully
            console.log('File loaded from event:', filePath);
          } else {
            console.log('Event payload format unexpected:', event.payload);
          }
        });
        
        // Remove catch-all listener as it's causing invalid event name error
        console.log('Event listeners ready to be set up...');
        
        const unlistenFns = await Promise.all([
          listen('menu-new-file', () => {
            console.log('Menu new file event received');
            handleNewFile();
          }),
          listen('menu-open-file', () => {
            console.log('Menu open file event received');
            openFile();
          }),
          listen('menu-save-file', () => {
            console.log('Menu save file event received');
            handleSaveFile();
          }),
          listen('menu-save-as-file', () => {
            console.log('Menu save as file event received');
            handleSaveAsFile();
          }),
          listen('menu-about', () => {
            console.log('Menu about event received');
            setShowAbout(true);
          }),
          listen('menu-debug-info', async () => {
            console.log('Menu debug info event received');
            if (debugInfo) {
              setDebugInfo('');
            } else {
              await debugArgs();
            }
          }),
          listen('menu-toggle-theme', () => {
            console.log('Menu toggle theme event received');
            toggleTheme();
          }),
          listen('menu-zoom-in', () => {
            console.log('Menu zoom in event received');
            handleZoomIn();
          }),
          listen('menu-zoom-out', () => {
            console.log('Menu zoom out event received');
            handleZoomOut();
          }),
          listen('menu-reset-zoom', () => {
            console.log('Menu reset zoom event received');
            handleResetZoom();
          }),
          listen('menu-undo', () => {
            console.log('Menu undo event received');
            handleUndo();
          }),
          listen('menu-redo', () => {
            console.log('Menu redo event received');
            handleRedo();
          }),
          listen('menu-cut', () => {
            console.log('Menu cut event received');
            handleCut();
          }),
          listen('menu-copy', () => {
            console.log('Menu copy event received');
            handleCopy();
          }),
          listen('menu-paste', () => {
            console.log('Menu paste event received');
            handlePaste();
          }),
          listen('menu-select-all', () => {
            console.log('Menu select all event received');
            handleSelectAll();
          }),
          // Add Tauri v2 drag-drop event listeners
          listen<string[]>('tauri://drag-drop', (event) => {
            console.log('Tauri drag-drop event received:', event);
            // This should be handled by the backend, but let's log it
          }),
          listen('tauri://drag-enter', (event) => {
            console.log('Tauri drag-enter event received:', event);
            setIsDragOver(true);
          }),
          listen('tauri://drag-leave', (event) => {
            console.log('Tauri drag-leave event received:', event);
            setIsDragOver(false);
          }),
          listen<string>('file-saved', (event) => {
            setCurrentFile(event.payload);
            setHasUnsavedChanges(false);
          }),
          listen<void>('file-new', () => {
            handleNewFile();
          }),
        ]);

        unlisten = () => {
          unlistenFns.forEach(fn => fn());
          fileOpenedListener();
        };

        console.log('All Tauri event listeners set up successfully');

      } catch (error) {
        console.error('Error setting up Tauri listeners:', error);
      }
    };

      console.log('About to setup listeners...');
      await setupListeners();
    };
    
    initializeApp();
    
    return () => {
      unlisten?.();
    };
  }, []);

  // Apply theme to document and update menu when theme changes
  useEffect(() => {
    console.log('🔄 useEffect triggered - isDarkTheme:', isDarkTheme, 'isTauri:', isTauri)
    
    if (isDarkTheme) {
      document.documentElement.setAttribute('data-theme', 'dark')
      console.log('📋 Applied dark theme to document')
    } else {
      document.documentElement.removeAttribute('data-theme')
      console.log('📋 Applied light theme to document')
    }
    
    // Update menu item text in Tauri when theme changes
    if (isTauri) {
      console.log('🍎 About to update menu with isDarkTheme:', isDarkTheme)
      const updateMenu = async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core')
          await invoke('update_theme_menu', { isDark: isDarkTheme })
          console.log('Menu updated - current theme:', isDarkTheme ? 'dark' : 'light', 'menu will show switch to:', isDarkTheme ? 'light' : 'dark')
        } catch (error) {
          console.error('Failed to update menu:', error)
        }
      }
      updateMenu()
    }
  }, [isDarkTheme, isTauri])

  const handleNewFile = () => {
    setContent('')
    setCurrentFile(null)
    setHasUnsavedChanges(false)
    console.log('New file created')
  }

  const openFile = async () => {
    console.log('openFile called, isTauri:', isTauri);
    if (!isTauri) {
      // Web mode - create file input element
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.md,.markdown,.txt';
      input.onchange = (e) => {
        const files = (e.target as HTMLInputElement).files;
        if (files && files.length > 0) {
          const file = files[0];
          const reader = new FileReader();
          reader.onload = (e) => {
            const content = e.target?.result as string;
            setContent(content);
            setCurrentFile(file.name);
            setHasUnsavedChanges(false);
            console.log('File loaded successfully:', file.name);
          };
          reader.readAsText(file);
        }
      };
      input.click();
    } else {
      // Tauri mode - use backend command
      try {
        console.log('Calling open_file_dialog command');
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('open_file_dialog');
        console.log('open_file_dialog command called successfully');
      } catch (error) {
        console.error("Error calling open_file_dialog command:", error);
        
        // Fallback to direct dialog usage
        try {
          console.log('Trying fallback method');
          const { open } = await import('@tauri-apps/plugin-dialog');
          const path = await open({
            multiple: false,
            filters: [{
              name: 'Markdown',
              extensions: ['md', 'markdown', 'txt']
            }]
          });

          if (typeof path === 'string') {
            const { readTextFile } = await import('@tauri-apps/plugin-fs');
            const contents = await readTextFile(path);
            setContent(contents);
            setCurrentFile(path);
            setHasUnsavedChanges(false);
            console.log('File opened successfully via fallback:', path);
          }
        } catch (fallbackError) {
          console.error("Fallback method also failed:", fallbackError);
        }
      }
    }
  };

  

  const handleSaveFile = async () => {
    if (isTauri) {
      // Use Tauri save
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        if (currentFile) {
          await invoke('save_file', { path: currentFile, content })
          setHasUnsavedChanges(false)
          console.log('File saved successfully')
        } else {
          await handleSaveAsFile()
        }
      } catch (error) {
        console.error('Error saving file:', error)
      }
    } else {
      // Use download for web
      handleSaveAsFile()
    }
  }

  const handleSaveAsFile = async () => {
    if (isTauri) {
      // Use Tauri save dialog
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const result = await invoke<string | null>('save_file_dialog', { content })
        if (result) {
          setCurrentFile(result)
          setHasUnsavedChanges(false)
          console.log('File saved as:', result)
        }
      } catch (error) {
        console.error('Error saving file as:', error)
      }
    } else {
      // Use download for web
      const blob = new Blob([content], { type: 'text/markdown' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = currentFile || 'document.md'
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      setHasUnsavedChanges(false)
      console.log('File downloaded:', currentFile || 'document.md')
    }
  }

  // This useEffect block was removed because its logic has been consolidated
  // into a single, robust listener setup at the top of the component.

  const handleContentChange = (newContent: string) => {
    setContent(newContent)
    setHasUnsavedChanges(true)
  }

  const toggleTheme = async () => {
    // Use the ref to get the current theme state (avoids stale closure)
    const currentTheme = isDarkThemeRef.current
    console.log('🎨 toggleTheme called - current isDarkTheme (from ref):', currentTheme)
    const newTheme = !currentTheme
    console.log('🎨 newTheme will be:', newTheme ? 'DARK' : 'LIGHT')
    
    // Update all the state and UI immediately with the new theme value
    console.log('🔄 Calling setIsDarkTheme with:', newTheme)
    setIsDarkTheme(newTheme)
    console.log('✅ setIsDarkTheme called - state should update to:', newTheme ? 'DARK' : 'LIGHT')
    
    localStorage.setItem('markdown-editor-theme', newTheme ? 'dark' : 'light')
    
    // Apply theme to the root element for CSS variables (backup)
    if (newTheme) {
      document.documentElement.setAttribute('data-theme', 'dark')
    } else {
      document.documentElement.removeAttribute('data-theme')
    }
    
    console.log('Theme toggled to:', newTheme ? 'dark' : 'light')
  }

  const handleZoomIn = () => {
    const currentZoom = parseFloat(document.body.style.zoom || '1')
    const newZoom = Math.min(currentZoom + 0.1, 2.0)
    document.body.style.zoom = newZoom.toString()
    console.log(`Zoomed in to ${Math.round(newZoom * 100)}%`)
  }

  const handleZoomOut = () => {
    const currentZoom = parseFloat(document.body.style.zoom || '1')
    const newZoom = Math.max(currentZoom - 0.1, 0.5)
    document.body.style.zoom = newZoom.toString()
    console.log(`Zoomed out to ${Math.round(newZoom * 100)}%`)
  }

  const handleResetZoom = () => {
    document.body.style.zoom = '1'
    console.log('Zoom reset to 100%')
  }

  // Menu-triggered editor operations (keyboard shortcuts handled by Monaco)
  const handleUndo = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      editorRef.current.trigger('menu', 'undo', null)
      console.log('Undo executed from menu')
    }
  }

  const handleRedo = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      editorRef.current.trigger('menu', 'redo', null)
      console.log('Redo executed from menu')
    }
  }

  const handleCut = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      editorRef.current.trigger('menu', 'editor.action.clipboardCutAction', null)
      console.log('Cut executed from menu')
    }
  }

  const handleCopy = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      editorRef.current.trigger('menu', 'editor.action.clipboardCopyAction', null)
      console.log('Copy executed from menu')
    }
  }

  const handlePaste = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      // Use Monaco's native paste action to work with WebKit
      editorRef.current.trigger('keyboard', 'editor.action.clipboardPasteAction', null)
      console.log('Paste executed from menu')
    }
  }

  const handleSelectAll = () => {
    if (editorRef.current) {
      editorRef.current.focus()
      editorRef.current.trigger('menu', 'editor.action.selectAll', null)
      console.log('Select all executed from menu')
    }
  }

  const debugArgs = async () => {
    if (isWeb) return
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const args = await invoke<string[]>('debug_args')
      console.log('Command line arguments:', args)
      const debugText = args.map((arg, i) => `${i}: ${arg}`).join('\n')
      setDebugInfo(debugText)
      alert('Command line arguments:\n' + debugText)
    } catch (error) {
      console.error('Error getting args:', error)
      setDebugInfo('Error: ' + error)
      alert('Error getting args: ' + error)
    }
  }

  // Extract document title from markdown content
  const extractDocumentTitle = (markdownContent: string): string | null => {
    if (!markdownContent.trim()) return null
    
    const lines = markdownContent.split('\n')
    for (const line of lines) {
      const trimmed = line.trim()
      if (trimmed.startsWith('# ')) {
        // Found an H1, extract the title
        return trimmed.substring(2).trim()
      }
      // Stop looking after we hit substantial content (not just whitespace or comments)
      if (trimmed && !trimmed.startsWith('<!--') && !trimmed.endsWith('-->')) {
        break
      }
    }
    return null
  }

  // Update window title when file changes (Tauri only)
  const updateWindowTitle = async (filePath: string | null, hasChanges: boolean) => {
    const displayTitle = documentTitle || (filePath ? filePath.split('/').pop() : 'Untitled Document')
    const changeIndicator = hasChanges ? ' •' : ''
    const fullTitle = `${displayTitle}${changeIndicator} - Mark-us-Down`

    if (!isTauri) {
      // In web mode, update document title
      document.title = fullTitle
      return
    }

    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const appWindow = getCurrentWindow()
      await appWindow.setTitle(fullTitle)
    } catch (error) {
      console.error('Failed to update window title:', error)
    }
  }

  // Update document title when content changes
  useEffect(() => {
    const title = extractDocumentTitle(content)
    setDocumentTitle(title)
  }, [content])

  // Update window title when file, changes state, or document title changes
  useEffect(() => {
    updateWindowTitle(currentFile, hasUnsavedChanges)
  }, [currentFile, hasUnsavedChanges, documentTitle])

  // Scroll synchronization handlers
  const handleEditorMount = (editor: monaco.editor.IStandaloneCodeEditor) => {
    scrollSyncService.setEditor(editor)
    editorRef.current = editor // Store reference for clipboard operations
  }

  const handlePreviewMount = (element: HTMLDivElement) => {
    scrollSyncService.setPreviewElement(element)
  }

  const handleEditorScroll = () => {
    scrollSyncService.syncEditorToPreview()
  }

  const handlePreviewScroll = () => {
    scrollSyncService.syncPreviewToEditor()
  }

  // Rebuild line mappings whenever content changes
  useEffect(() => {
    // Give the preview time to render
    const timer = setTimeout(() => {
      scrollSyncService.buildLineMappings()
    }, 100)
    
    return () => clearTimeout(timer)
  }, [content])

  // Drag and drop visual feedback handlers
  // File handling is done by Tauri backend, these are just for UI feedback
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault() // Always prevent default for visual feedback
    e.stopPropagation()
    if (!isDragOver) setIsDragOver(true)
  }

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(true)
  }

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    // Only set drag over to false if we're leaving the app container
    if (e.currentTarget === e.target) {
      setIsDragOver(false)
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false) // Always clear the visual state
    // File handling is done by Tauri backend - no need to process files here
  }

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
      const modifier = isMac ? e.metaKey : e.ctrlKey

      if (modifier) {
        switch (e.key.toLowerCase()) {
          case 'n':
            e.preventDefault()
            handleNewFile()
            break
          case 'o':
            e.preventDefault()
            openFile()
            break
          case 's':
            e.preventDefault()
            if (e.shiftKey) {
              handleSaveAsFile()
            } else {
              handleSaveFile()
            }
            break
          case 't':
            e.preventDefault()
            toggleTheme()
            break
          // Let Monaco handle clipboard operations via keyboard shortcuts
          // These handlers are only for menu items now
          case '=':
          case '+':
            e.preventDefault()
            handleZoomIn()
            break
          case '-':
            e.preventDefault()
            handleZoomOut()
            break
          case '0':
            e.preventDefault()
            handleResetZoom()
            break
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [content, currentFile])

  // Clean up on unmount
  useEffect(() => {
    return () => {
      scrollSyncService.dispose()
    }
  }, [])


  return (
    <div 
      className={`App ${isDragOver ? 'drag-over' : ''}`}
      onDragOver={handleDragOver}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <header className="App-header">
        <div className="file-info">
          <div className="file-status">
            <span className="file-name">
              {documentTitle || 'Untitled Document'}
              {hasUnsavedChanges && ' •'}
            </span>
            <span className="file-path">
              {currentFile ? currentFile.split('/').pop() : 'No file loaded'}
            </span>
          </div>
        </div>
        <div className="controls">
          {isWeb && (
            <>
              <button onClick={handleNewFile}>New</button>
              <button onClick={openFile}>Load</button>
              <button onClick={handleSaveFile}>Save</button>
              <button onClick={() => setShowAbout(true)}>About</button>
              <button onClick={toggleTheme} className="theme-toggle">
                {isDarkTheme ? '☀️' : '🌙'}
              </button>
            </>
          )}
        </div>
      </header>
      <main className="editor-container">
        <SplitView
          leftComponent={
            <SourceEditor
              value={content}
              onChange={handleContentChange}
              theme={isDarkTheme ? 'dark' : 'light'}
              onScroll={handleEditorScroll}
              onEditorMount={handleEditorMount}
            />
          }
          rightComponent={
            <PreviewPane 
              content={content}
              onScroll={handlePreviewScroll}
              onMount={handlePreviewMount}
            />
          }
        />
      </main>

      {/* About Dialog */}
      {showAbout && (
        <div className="modal-overlay" onClick={() => setShowAbout(false)}>
          <div className="modal-content about-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="about-header">
              <img src="/ME_Logo192.png" alt="Mark-us-Down Logo" className="about-logo" />
              <h2>Mark-us-Down</h2>
            </div>
            <p className="version">Version 1.0.0</p>
            <p className="tagline">A modern, split-pane markdown editor with real-time preview</p>
            <div className="about-links">
              <a 
                href="https://brightlight.rocks" 
                onClick={async (e) => {
                  e.preventDefault();
                  if (!isWeb) {
                    try {
                      const { open } = await import('@tauri-apps/plugin-shell');
                      await open('https://brightlight.rocks');
                    } catch (error) {
                      console.error('Failed to open URL:', error);
                    }
                  } else {
                    window.open('https://brightlight.rocks', '_blank');
                  }
                }}
              >
                https://brightlight.rocks
              </a>
            </div>
            <p className="copyright">Copyright © 2025 Bright Light Interstellar Ltd</p>
            <div className="modal-buttons">
              <button onClick={() => setShowAbout(false)}>Close</button>
            </div>
          </div>
        </div>
      )}

      {/* Debug Info Display */}
      {debugInfo && !isWeb && (
        <div className="debug-info-box">
          <div className="debug-info-header">
            <strong>Debug Info:</strong>
            <button onClick={() => setDebugInfo('')} className="debug-info-close">×</button>
          </div>
          <pre className="debug-info-content">{debugInfo}</pre>
        </div>
      )}
    </div>
  )
}

export default App