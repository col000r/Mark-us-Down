import * as monaco from 'monaco-editor'

interface LineMapping {
  sourceLine: number
  previewTop: number
}

export class ScrollSyncService {
  private editorInstance: monaco.editor.IStandaloneCodeEditor | null = null
  private previewElement: HTMLElement | null = null
  private isEditorScrolling = false
  private isPreviewScrolling = false
  private lineMappings: LineMapping[] = []

  setEditor(editor: monaco.editor.IStandaloneCodeEditor) {
    this.editorInstance = editor
  }

  setPreviewElement(element: HTMLElement) {
    this.previewElement = element
  }

  /**
   * Build line mappings between source lines and preview positions
   * This helps with accurate scroll synchronization
   */
  buildLineMappings() {
    if (!this.previewElement || !this.editorInstance) return

    this.lineMappings = []

    // Find all elements with line number data attributes
    const lineElements = this.previewElement.querySelectorAll('[data-source-line]')

    lineElements.forEach((element) => {
      const sourceLine = parseInt(element.getAttribute('data-source-line') || '0', 10)
      const rect = element.getBoundingClientRect()
      const previewRect = this.previewElement!.getBoundingClientRect()
      const relativeTop = rect.top - previewRect.top + this.previewElement!.scrollTop

      this.lineMappings.push({
        sourceLine,
        previewTop: relativeTop
      })
    })

    // Sort by source line for binary search
    this.lineMappings.sort((a, b) => a.sourceLine - b.sourceLine)
  }

  /**
   * Get the preview scroll position for a given editor line
   */
  private getPreviewPositionForEditorLine(line: number): number {
    if (this.lineMappings.length === 0) {
      // Fallback to percentage-based sync
      const totalLines = this.editorInstance?.getModel()?.getLineCount() || 1
      const percentage = line / totalLines
      return percentage * (this.previewElement?.scrollHeight || 0)
    }

    // Find the closest mapping
    let left = 0
    let right = this.lineMappings.length - 1

    while (left <= right) {
      const mid = Math.floor((left + right) / 2)
      if (this.lineMappings[mid].sourceLine === line) {
        return this.lineMappings[mid].previewTop
      } else if (this.lineMappings[mid].sourceLine < line) {
        left = mid + 1
      } else {
        right = mid - 1
      }
    }

    // Interpolate between closest mappings
    if (right >= 0 && left < this.lineMappings.length) {
      const before = this.lineMappings[right]
      const after = this.lineMappings[left]
      const ratio = (line - before.sourceLine) / (after.sourceLine - before.sourceLine)
      return before.previewTop + ratio * (after.previewTop - before.previewTop)
    }

    // Fallback to percentage
    const totalLines = this.editorInstance?.getModel()?.getLineCount() || 1
    const percentage = line / totalLines
    return percentage * (this.previewElement?.scrollHeight || 0)
  }

  /**
   * Get the editor line for a given preview scroll position
   */
  private getEditorLineForPreviewPosition(scrollTop: number): number {
    if (this.lineMappings.length === 0) {
      // Fallback to percentage-based sync
      const percentage = scrollTop / (this.previewElement?.scrollHeight || 1)
      const totalLines = this.editorInstance?.getModel()?.getLineCount() || 1
      return Math.round(percentage * totalLines)
    }

    // Find the closest mapping
    for (let i = this.lineMappings.length - 1; i >= 0; i--) {
      if (this.lineMappings[i].previewTop <= scrollTop) {
        if (i === this.lineMappings.length - 1) {
          return this.lineMappings[i].sourceLine
        }

        // Interpolate
        const before = this.lineMappings[i]
        const after = this.lineMappings[i + 1]
        const ratio = (scrollTop - before.previewTop) / (after.previewTop - before.previewTop)
        return Math.round(before.sourceLine + ratio * (after.sourceLine - before.sourceLine))
      }
    }

    return 1
  }

  /**
   * Sync scroll from editor to preview - content-aware with line mappings
   */
  syncEditorToPreview() {
    // Prevent loop: if we're already syncing from preview, don't sync back
    if (this.isPreviewScrolling) {
      return
    }

    if (!this.editorInstance || !this.previewElement) {
      return
    }

    this.isEditorScrolling = true

    const visibleRange = this.editorInstance.getVisibleRanges()[0]
    if (!visibleRange) {
      this.isEditorScrolling = false
      return
    }

    const topLine = visibleRange.startLineNumber
    const scrollTop = this.getPreviewPositionForEditorLine(topLine)

    // Adjust preview scroll (instant, no smooth scroll to avoid loop issues)
    this.previewElement.scrollTop = Math.max(0, scrollTop)

    // Clear the flag after a short delay
    setTimeout(() => {
      this.isEditorScrolling = false
    }, 50)
  }

  /**
   * Sync scroll from preview to editor - content-aware with line mappings
   */
  syncPreviewToEditor() {
    // Prevent loop: if we're already syncing from editor, don't sync back
    if (this.isEditorScrolling) {
      return
    }

    if (!this.editorInstance || !this.previewElement) {
      return
    }

    this.isPreviewScrolling = true

    const scrollTop = this.previewElement.scrollTop
    const line = this.getEditorLineForPreviewPosition(scrollTop)

    // Scroll editor to the line (instant, no smooth scroll to avoid loop issues)
    this.editorInstance.setScrollTop(
      this.editorInstance.getTopForLineNumber(line)
    )

    // Clear the flag after a short delay
    setTimeout(() => {
      this.isPreviewScrolling = false
    }, 50)
  }

  /**
   * Clean up resources
   */
  dispose() {
    this.editorInstance = null
    this.previewElement = null
    this.lineMappings = []
  }
}

export const scrollSyncService = new ScrollSyncService()
