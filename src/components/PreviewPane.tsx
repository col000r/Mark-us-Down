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

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    // Call onMount callback
    if (onMount) {
      onMount(container)
    }

    // Set up scroll event listener
    if (onScroll) {
      const handleScroll = () => {
        onScroll()
      }
      
      container.addEventListener('scroll', handleScroll)
      
      return () => {
        container.removeEventListener('scroll', handleScroll)
      }
    }
  }, [onMount, onScroll])

  return (
    <div 
      ref={containerRef}
      className={`preview-pane ${className}`}
    >
      <div 
        className="preview-content"
        dangerouslySetInnerHTML={{ __html: htmlContent }}
      />
    </div>
  )
}