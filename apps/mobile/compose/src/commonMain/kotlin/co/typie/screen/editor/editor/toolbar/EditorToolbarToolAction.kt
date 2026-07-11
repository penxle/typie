package co.typie.screen.editor.editor.toolbar

internal enum class EditorToolbarToolAction {
  Search,
  RelatedNotes,
  Comment,
  Spellcheck,
  AiFeedback,
  Timeline,
  DebugViewportOverlay,
  DebugBodyOverlay,
  DebugSurfaceOverlay,
  SendInputLog,
}

internal data class EditorToolbarDebugOverlays(
  val viewportVisible: Boolean,
  val bodyVisible: Boolean,
  val surfaceVisible: Boolean,
  val inputLogAvailable: Boolean,
)
