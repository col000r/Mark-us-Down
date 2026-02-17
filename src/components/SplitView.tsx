import React, { useState, useRef, useCallback, useEffect } from 'react'
import './SplitView.css'

interface SplitViewProps {
  leftComponent: React.ReactNode
  rightComponent: React.ReactNode
  className?: string
  initialRatio?: number
  hideLeft?: boolean
}

const DIVIDER_WIDTH = 4 // px - must match CSS .split-divider width

export const SplitView: React.FC<SplitViewProps> = ({
  leftComponent,
  rightComponent,
  className = '',
  initialRatio = 0.5,
  hideLeft = false
}) => {
  const [leftRatio, setLeftRatio] = useState(() => {
    const saved = localStorage.getItem('split-view-ratio')
    return saved ? parseFloat(saved) : initialRatio
  })
  const [isDragging, setIsDragging] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging || !containerRef.current) return

    const container = containerRef.current
    const rect = container.getBoundingClientRect()
    const newRatio = Math.max(0.1, Math.min(0.9, (e.clientX - rect.left) / rect.width))
    
    setLeftRatio(newRatio)
    localStorage.setItem('split-view-ratio', newRatio.toString())
  }, [isDragging])

  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = 'col-resize'
      document.body.style.userSelect = 'none'
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }
  }, [isDragging, handleMouseMove, handleMouseUp])

  return (
    <div
      ref={containerRef}
      className={`split-view ${className} ${isDragging ? 'dragging' : ''} ${hideLeft ? 'reading-mode' : ''}`}
    >
      {!hideLeft && (
        <>
          <div
            className="split-pane split-pane-left"
            style={{ width: `calc(${leftRatio * 100}% - ${leftRatio * DIVIDER_WIDTH}px)` }}
          >
            {leftComponent}
          </div>

          <div
            className="split-divider"
            onMouseDown={handleMouseDown}
          >
            <div className="split-handle" />
          </div>
        </>
      )}

      <div
        className="split-pane split-pane-right"
        style={{ width: hideLeft ? '100%' : `calc(${(1 - leftRatio) * 100}% - ${(1 - leftRatio) * DIVIDER_WIDTH}px)` }}
      >
        {rightComponent}
      </div>
    </div>
  )
}