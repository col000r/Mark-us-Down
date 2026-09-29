import React from 'react'

interface StatusBarProps {
  filePath: string | null
  hasUnsavedChanges: boolean
  wordCount: number
  cursorLine: number
  cursorColumn: number
  isReadingMode: boolean
  onToggleReadingMode: () => void
}

export const StatusBar: React.FC<StatusBarProps> = ({
  filePath,
  hasUnsavedChanges,
  wordCount,
  cursorLine,
  cursorColumn,
  isReadingMode,
  onToggleReadingMode
}) => {
  return (
    <footer className="status-bar">
      <span className="status-path" title={filePath ?? undefined}>
        {/* Wrapped in <bdi> so the RTL-ellipsis trick keeps the path's own direction */}
        <bdi>{filePath ?? 'Not saved yet'}</bdi>
      </span>
      {hasUnsavedChanges && <span className="status-item status-edited">Edited</span>}
      <span className="status-spacer" />
      {!isReadingMode && (
        <span className="status-item">Ln {cursorLine}, Col {cursorColumn}</span>
      )}
      <span className="status-item">
        {wordCount.toLocaleString()} {wordCount === 1 ? 'word' : 'words'}
      </span>
      <button
        className={`status-item status-button ${isReadingMode ? 'active' : ''}`}
        onClick={onToggleReadingMode}
        title="Toggle Reading Mode (⌘E)"
      >
        Reading Mode
      </button>
    </footer>
  )
}
