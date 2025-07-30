import * as monaco from 'monaco-editor'

interface LineMapping {
  sourceLine: number
  sourceLineEnd?: number
  previewTop: number
  previewHeight?: number
}

export class ScrollSyncService {
  private editorInstance: monaco.editor.IStandaloneCodeEditor | null = null
  private previewElement: HTMLElement | null = null
  private isScrolling = false
  private scrollTimeout: number | null = null
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
    const denseMappings: LineMapping[] = []
    
    // Find all elements with line number data attributes
    const lineElements = this.previewElement.querySelectorAll('[data-source-line]')
    
    lineElements.forEach((element) => {
      const sourceLine = parseInt(element.getAttribute('data-source-line') || '0', 10)
      const sourceLineEnd = element.getAttribute('data-source-line-end')
      const rect = element.getBoundingClientRect()
      const previewRect = this.previewElement!.getBoundingClientRect()
      const relativeTop = rect.top - previewRect.top + this.previewElement!.scrollTop
      
      const mapping: LineMapping = {
        sourceLine,
        previewTop: relativeTop,
        previewHeight: rect.height
      }
      
      if (sourceLineEnd) {
        mapping.sourceLineEnd = parseInt(sourceLineEnd, 10)
      }
      
      denseMappings.push(mapping)
    })

    // Sort by source line for binary search
    denseMappings.sort((a, b) => a.sourceLine - b.sourceLine)
    
    // Create dense mappings by interpolating within elements that span multiple lines
    denseMappings.forEach(mapping => {
      if (mapping.sourceLineEnd && mapping.sourceLineEnd > mapping.sourceLine) {
        // This element spans multiple lines, create mapping for each line
        const linesSpanned = mapping.sourceLineEnd - mapping.sourceLine + 1
        for (let i = 0; i < linesSpanned; i++) {
          const lineNumber = mapping.sourceLine + i
          const lineProgress = i / Math.max(1, linesSpanned - 1)
          const lineTop = mapping.previewTop + (lineProgress * (mapping.previewHeight || 0))
          
          this.lineMappings.push({
            sourceLine: lineNumber,
            previewTop: lineTop
          })
        }
      } else {
        // Single line mapping
        this.lineMappings.push({
          sourceLine: mapping.sourceLine,
          previewTop: mapping.previewTop
        })
      }
    })
    
    // Sort by source line for binary search
    this.lineMappings.sort((a, b) => a.sourceLine - b.sourceLine)
    
    // Fill in gaps with linear interpolation
    this.fillMappingGaps()
    
    // Debug: Log mapping summary
    if (this.lineMappings.length > 0) {
      console.log(`[ScrollSync] Built ${this.lineMappings.length} line mappings for scroll sync`)
    }
  }

  /**
   * Fill gaps in line mappings with interpolated values
   */
  private fillMappingGaps() {
    if (this.lineMappings.length < 2) return
    
    const totalSourceLines = this.editorInstance?.getModel()?.getLineCount() || 0
    const filledMappings: LineMapping[] = []
    
    // Add mapping for line 1 if missing
    if (this.lineMappings[0].sourceLine > 1) {
      filledMappings.push({
        sourceLine: 1,
        previewTop: 0
      })
    }
    
    for (let i = 0; i < this.lineMappings.length - 1; i++) {
      const current = this.lineMappings[i]
      const next = this.lineMappings[i + 1]
      
      filledMappings.push(current)
      
      // Fill gaps between current and next
      const gap = next.sourceLine - current.sourceLine
      if (gap > 1) {
        for (let line = current.sourceLine + 1; line < next.sourceLine; line++) {
          const progress = (line - current.sourceLine) / gap
          const interpolatedTop = current.previewTop + progress * (next.previewTop - current.previewTop)
          filledMappings.push({
            sourceLine: line,
            previewTop: interpolatedTop
          })
        }
      }
    }
    
    // Add the last mapping
    filledMappings.push(this.lineMappings[this.lineMappings.length - 1])
    
    // Add mapping for the last line if missing
    const lastMapping = filledMappings[filledMappings.length - 1]
    if (lastMapping.sourceLine < totalSourceLines) {
      const previewHeight = this.previewElement?.scrollHeight || 0
      filledMappings.push({
        sourceLine: totalSourceLines,
        previewTop: previewHeight
      })
    }
    
    this.lineMappings = filledMappings
  }


  /**
   * Sync scroll from editor to preview - align top edges
   */
  syncEditorToPreview() {
    if (!this.editorInstance || !this.previewElement || this.isScrolling) return

    this.isScrolling = true
    
    const editorScrollTop = this.editorInstance.getScrollTop()
    const editorScrollHeight = this.editorInstance.getScrollHeight()
    const editorHeight = this.editorInstance.getLayoutInfo().height
    const editorMaxScroll = editorScrollHeight - editorHeight
    const previewScrollHeight = this.previewElement.scrollHeight
    const previewClientHeight = this.previewElement.clientHeight
    const previewMaxScroll = previewScrollHeight - previewClientHeight
    
    // Calculate percentage-based sync
    const editorPercentage = editorMaxScroll > 0 ? editorScrollTop / editorMaxScroll : 0
    const percentageBasedPreviewScroll = editorPercentage * previewMaxScroll
    
    // Check if editor is at the bottom
    const isEditorAtBottom = editorScrollTop >= editorMaxScroll - 5
    
    const targetScrollTop = isEditorAtBottom ? previewMaxScroll : percentageBasedPreviewScroll
    const finalScroll = Math.max(0, Math.min(targetScrollTop, previewMaxScroll))
    
    // Set scroll position directly (no smooth scrolling to prevent interference)
    this.previewElement.scrollTop = finalScroll

    // Reset scrolling flag immediately for direct scrolling
    if (this.scrollTimeout) clearTimeout(this.scrollTimeout)
    this.scrollTimeout = setTimeout(() => {
      this.isScrolling = false
    }, 50)
  }

  /**
   * Sync scroll from preview to editor - align top edges
   */
  syncPreviewToEditor() {
    if (!this.editorInstance || !this.previewElement || this.isScrolling) return

    this.isScrolling = true
    
    const scrollTop = this.previewElement.scrollTop
    const scrollHeight = this.previewElement.scrollHeight
    const clientHeight = this.previewElement.clientHeight
    const editorHeight = this.editorInstance.getLayoutInfo().height
    const editorScrollHeight = this.editorInstance.getScrollHeight()
    const editorMaxScroll = editorScrollHeight - editorHeight
    
    // Calculate percentage-based sync 
    const previewScrollableHeight = Math.max(1, scrollHeight - clientHeight)
    const previewPercentage = scrollTop / previewScrollableHeight
    const percentageBasedEditorScroll = previewPercentage * editorMaxScroll
    
    // Check if we're at the bottom of the preview
    const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10
    
    const targetScrollTop = isAtBottom ? editorMaxScroll : percentageBasedEditorScroll
    const finalScroll = Math.max(0, Math.min(targetScrollTop, editorMaxScroll))
    
    // Set scroll position directly (no smooth scrolling to prevent interference)
    this.editorInstance.setScrollTop(finalScroll)

    // Reset scrolling flag immediately for direct scrolling  
    if (this.scrollTimeout) clearTimeout(this.scrollTimeout)
    this.scrollTimeout = setTimeout(() => {
      this.isScrolling = false
    }, 50)
  }

  /**
   * Clean up resources
   */
  dispose() {
    if (this.scrollTimeout) {
      clearTimeout(this.scrollTimeout)
    }
    this.editorInstance = null
    this.previewElement = null
    this.lineMappings = []
  }
}

export const scrollSyncService = new ScrollSyncService()