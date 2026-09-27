import React, { useState, useRef, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { markdownParser } from '../services/markdownParser'
import './PreviewPane.css'

interface PreviewPaneProps {
  content: string
  currentFile?: string | null
  className?: string
  onScroll?: () => void
  onMount?: (element: HTMLDivElement) => void
}

function getImageMimeType(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase() ?? ''
  const types: Record<string, string> = {
    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg',
    gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml',
    bmp: 'image/bmp', ico: 'image/x-icon',
  }
  return types[ext] ?? 'image/png'
}

export const PreviewPane: React.FC<PreviewPaneProps> = ({
  content,
  currentFile = null,
  className = '',
  onScroll,
  onMount
}) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const hasCalledMount = useRef(false)
  const [htmlContent, setHtmlContent] = useState('<div class="preview-placeholder">Start typing to see preview...</div>')

  useEffect(() => {
    if (!content.trim()) {
      setHtmlContent('<div class="preview-placeholder">Start typing to see preview...</div>')
      return
    }

    let parsed: string
    try {
      parsed = markdownParser.parse(content)
    } catch (error) {
      console.error('Error parsing markdown:', error)
      setHtmlContent('<div class="preview-error">Error rendering preview</div>')
      return
    }

    // Resolve relative image paths to base64 data URLs
    const docDir = currentFile ? currentFile.substring(0, currentFile.lastIndexOf('/')) : null
    if (!docDir) {
      setHtmlContent(parsed)
      return
    }

    const imgRegex = /<img([^>]*)\ssrc="([^"]+)"([^>]*)>/gi
    const relativeSrcs: string[] = []
    let match: RegExpExecArray | null
    while ((match = imgRegex.exec(parsed)) !== null) {
      const src = match[2]
      if (!src.startsWith('http://') && !src.startsWith('https://') && !src.startsWith('data:') && !src.startsWith('/')) {
        relativeSrcs.push(src)
      }
    }

    if (relativeSrcs.length === 0) {
      setHtmlContent(parsed)
      return
    }

    // Load all relative images as base64 in parallel
    Promise.all(
      relativeSrcs.map(async (src) => {
        // src is HTML-serialized (&amp;) and URL-encoded (%20) — decode both to get the real file path
        let cleanSrc = src.replace(/&amp;/g, '&')
        try {
          cleanSrc = decodeURIComponent(cleanSrc)
        } catch {
          // Malformed escape sequence (e.g. a literal "%" in the filename) — use as-is
        }
        if (cleanSrc.startsWith('./')) cleanSrc = cleanSrc.slice(2)
        const absolutePath = `${docDir}/${cleanSrc}`
        try {
          const base64 = await invoke<string>('read_binary_file', { path: absolutePath })
          return { src, dataUrl: `data:${getImageMimeType(src)};base64,${base64}` }
        } catch (e) {
          console.warn(`Could not load image: ${absolutePath}`, e)
          return null
        }
      })
    ).then((results) => {
      let html = parsed
      for (const result of results) {
        if (result) {
          html = html.split(`src="${result.src}"`).join(`src="${result.dataUrl}"`)
        }
      }
      setHtmlContent(html)
    })
  }, [content, currentFile])

  // Call onMount once when container is ready
  useEffect(() => {
    if (containerRef.current && onMount && !hasCalledMount.current) {
      console.log('[PreviewPane] Calling onMount')
      onMount(containerRef.current)
      hasCalledMount.current = true
    }
  }, [onMount])

  // Simple scroll handler
  const handleScroll = () => {
    console.log('[PreviewPane] handleScroll called')
    if (onScroll) {
      onScroll()
    }
  }

  return (
    <div
      ref={containerRef}
      className={`preview-pane ${className}`}
      onScroll={handleScroll}
    >
      <div
        className="preview-content"
        dangerouslySetInnerHTML={{ __html: htmlContent }}
      />
    </div>
  )
}
