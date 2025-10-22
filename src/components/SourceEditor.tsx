import React, { useRef, useCallback, useEffect } from 'react'
import Editor, { OnMount } from '@monaco-editor/react'
import * as monaco from 'monaco-editor'
import './SourceEditor.css'

interface SourceEditorProps {
  value: string
  onChange: (value: string) => void
  readOnly?: boolean
  className?: string
  theme?: 'light' | 'dark'
  onScroll?: () => void
  onEditorMount?: (editor: monaco.editor.IStandaloneCodeEditor) => void
}

export const SourceEditor: React.FC<SourceEditorProps> = ({
  value,
  onChange,
  readOnly = false,
  className = '',
  theme = 'dark',
  onScroll,
  onEditorMount
}) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const onScrollRef = useRef(onScroll)

  // Keep scroll ref updated
  useEffect(() => {
    onScrollRef.current = onScroll
  }, [onScroll])

  const handleEditorDidMount: OnMount = useCallback((editor, monaco) => {
    editorRef.current = editor

    // Configure Monaco editor themes
    monaco.editor.defineTheme('markdown-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: '60a5fa' },
        { token: 'string', foreground: 'a78bfa' },
        { token: 'comment', foreground: '6b7280' },
        { token: 'number', foreground: '34d399' },
        { token: 'variable', foreground: 'e4e4e7' },
        { token: 'type', foreground: 'fbbf24' },
      ],
      colors: {
        'editor.background': '#1e1e1e',
        'editor.foreground': '#e4e4e7',
        'editor.lineHighlightBackground': '#2a2a2a',
        'editor.selectionBackground': '#3b82f6',
        'editorCursor.foreground': '#60a5fa',
        'editor.selectionHighlightBackground': '#374151',
        'editorLineNumber.foreground': '#6b7280',
        'editorLineNumber.activeForeground': '#9ca3af',
      }
    })

    monaco.editor.defineTheme('markdown-light', {
      base: 'vs',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: '0000ff' },
        { token: 'string', foreground: 'a31515' },
        { token: 'comment', foreground: '008000' },
        { token: 'number', foreground: '09885a' },
      ],
      colors: {
        'editor.background': '#ffffff',
        'editor.foreground': '#000000',
        'editor.lineHighlightBackground': '#f5f5f5',
        'editor.selectionBackground': '#add6ff',
        'editorCursor.foreground': '#000000',
      }
    })

    monaco.editor.setTheme(theme === 'dark' ? 'markdown-dark' : 'markdown-light')

    // Configure editor options
    editor.updateOptions({
      fontSize: 14,
      fontFamily: 'Monaco, Menlo, "Ubuntu Mono", monospace',
      lineHeight: 21,
      wordWrap: 'on',
      wrappingIndent: 'indent',
      lineNumbers: 'on',
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      automaticLayout: true,
      renderWhitespace: 'selection',
      folding: true,
      foldingStrategy: 'indentation',
      showFoldingControls: 'mouseover',
      contextmenu: false, // Disable Monaco's context menu
      quickSuggestions: false, // Disable suggestions that might interfere
      wordBasedSuggestions: 'off', // Disable word-based suggestions
      suggestOnTriggerCharacters: false, // Disable auto-suggest
      acceptSuggestionOnEnter: 'off', // Disable suggestion on enter
      tabCompletion: 'off', // Disable tab completion
      parameterHints: { enabled: false }, // Disable parameter hints
      hover: { enabled: false }, // Disable hover tooltips
      lightbulb: { enabled: monaco.editor.ShowLightbulbIconMode.Off }, // Disable lightbulb suggestions
    })


    // Add scroll event listener - always set up, use ref for callback
    console.log('[SourceEditor] Setting up scroll listener')
    editor.onDidScrollChange(() => {
      console.log('[SourceEditor] Scroll event fired')
      if (onScrollRef.current) {
        onScrollRef.current()
      }
    })

    // Call the onEditorMount callback if provided
    if (onEditorMount) {
      onEditorMount(editor)
    }
  }, [theme, onEditorMount])

  // Theme is now handled by the Editor component directly

  const handleEditorChange = useCallback((value: string | undefined) => {
    if (value !== undefined) {
      onChange(value)
    }
  }, [onChange])

  return (
    <div className={`source-editor ${className}`}>
      <Editor
        height="100%"
        language="markdown"
        value={value}
        onChange={handleEditorChange}
        onMount={handleEditorDidMount}
        theme={theme === 'dark' ? 'markdown-dark' : 'markdown-light'}
        beforeMount={(monaco) => {
          // Completely disable dropOrPasteInto feature
          try {
            const originalCreateEditor = monaco.editor.create;
            monaco.editor.create = function(container, options, overrides) {
              const modifiedOptions = {
                ...options,
                'dropOrPasteInto.enabled': false,
                'editor.dropOrPasteInto.enabled': false
              };
              return originalCreateEditor.call(this, container, modifiedOptions, overrides);
            };
          } catch (e) {
            console.log('Could not override editor creation:', e);
          }
        }}
        options={{
          readOnly,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          lineNumbers: 'on',
          renderWhitespace: 'selection',
          wordWrap: 'on',
          automaticLayout: true,
          contextmenu: false, // Disable context menu
          quickSuggestions: false, // Disable suggestions
          wordBasedSuggestions: 'off', // Disable word-based suggestions
          suggestOnTriggerCharacters: false, // Disable auto-suggest
          acceptSuggestionOnEnter: 'off', // Disable suggestion on enter
          tabCompletion: 'off', // Disable tab completion
          parameterHints: { enabled: false }, // Disable parameter hints
          hover: { enabled: false }, // Disable hover tooltips
          lightbulb: { enabled: monaco.editor.ShowLightbulbIconMode.Off }, // Disable lightbulb suggestions
          dropIntoEditor: { enabled: false }, // Disable drop into editor
        }}
      />
    </div>
  )
}