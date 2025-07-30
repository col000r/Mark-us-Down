import React, { useState, useRef, useCallback, useEffect } from 'react'
import './SplitView.css'

interface SplitViewProps {
  leftComponent: React.ReactNode
  rightComponent: React.ReactNode
  className?: string
  initialRatio?: number
}

export const SplitView: React.FC<SplitViewProps> = ({
  leftComponent,
  rightComponent,
  className = '',
  initialRatio = 0.5
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
      className={`split-view ${className} ${isDragging ? 'dragging' : ''}`}
    >
      <div 
        className="split-pane split-pane-left"
        style={{ width: `${leftRatio * 100}%` }}
      >
        {leftComponent}
      </div>
      
      <div 
        className="split-divider"
        onMouseDown={handleMouseDown}
      >
        <div className="split-handle" />
      </div>
      
      <div 
        className="split-pane split-pane-right"
        style={{ width: `${(1 - leftRatio) * 100}%` }}
      >
        {rightComponent}
      </div>
    </div>
  )
}