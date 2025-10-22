import React, { useMemo, useRef, useEffect } from 'react'
import { markdownParser } from '../services/markdownParser'
import './PreviewPane.css'

interface PreviewPaneProps {
  content: string
  className?: string
  onScroll?: () => void
  onMount?: (element: HTMLDivElement) => void
}

export const PreviewPane: React.FC<PreviewPaneProps> = ({
  content,
  className = '',
  onScroll,
  onMount
}) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const hasCalledMount = useRef(false)

  const htmlContent = useMemo(() => {
    if (!content.trim()) {
      return '<div class="preview-placeholder">Start typing to see preview...</div>'
    }

    try {
      return markdownParser.parse(content)
    } catch (error) {
      console.error('Error parsing markdown:', error)
      return '<div class="preview-error">Error rendering preview</div>'
    }
  }, [content])

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
