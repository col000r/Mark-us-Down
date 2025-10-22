import * as monaco from 'monaco-editor'

export class ScrollSyncService {
  private editorInstance: monaco.editor.IStandaloneCodeEditor | null = null
  private previewElement: HTMLElement | null = null
  private isEditorScrolling = false
  private isPreviewScrolling = false

  constructor() {
    // Service initialized
  }

  setEditor(editor: monaco.editor.IStandaloneCodeEditor) {
    this.editorInstance = editor
  }

  setPreviewElement(element: HTMLElement) {
    this.previewElement = element
  }

  buildLineMappings() {
    // Not used in simple version
  }

  /**
   * Sync scroll from editor to preview - simple percentage-based
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

    const editorScrollTop = this.editorInstance.getScrollTop()
    const editorScrollHeight = this.editorInstance.getScrollHeight()
    const editorHeight = this.editorInstance.getLayoutInfo().height
    const editorMaxScroll = Math.max(1, editorScrollHeight - editorHeight)

    const previewScrollHeight = this.previewElement.scrollHeight
    const previewClientHeight = this.previewElement.clientHeight
    const previewMaxScroll = Math.max(1, previewScrollHeight - previewClientHeight)

    // Calculate percentage
    const percentage = editorScrollTop / editorMaxScroll
    const targetScroll = percentage * previewMaxScroll

    this.previewElement.scrollTop = targetScroll

    // Clear the flag after a short delay to allow the scroll to complete
    setTimeout(() => {
      this.isEditorScrolling = false
    }, 50)
  }

  /**
   * Sync scroll from preview to editor - simple percentage-based
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

    const previewScrollTop = this.previewElement.scrollTop
    const previewScrollHeight = this.previewElement.scrollHeight
    const previewClientHeight = this.previewElement.clientHeight
    const previewMaxScroll = Math.max(1, previewScrollHeight - previewClientHeight)

    const editorScrollHeight = this.editorInstance.getScrollHeight()
    const editorHeight = this.editorInstance.getLayoutInfo().height
    const editorMaxScroll = Math.max(1, editorScrollHeight - editorHeight)

    // Calculate percentage
    const percentage = previewScrollTop / previewMaxScroll
    const targetScroll = percentage * editorMaxScroll

    this.editorInstance.setScrollTop(targetScroll)

    // Clear the flag after a short delay to allow the scroll to complete
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
  }
}

export const scrollSyncService = new ScrollSyncService()
