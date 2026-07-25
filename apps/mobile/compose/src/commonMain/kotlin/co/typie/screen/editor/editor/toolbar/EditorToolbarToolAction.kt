package co.typie.screen.editor.editor.toolbar

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

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

internal data class EditorToolbarToolItem(
  val icon: IconData,
  val label: String,
  val action: EditorToolbarToolAction,
  val key: String = label,
)

internal val EditorToolbarToolItems =
  listOf(
    EditorToolbarToolItem(
      icon = Lucide.StickyNote,
      label = "노트",
      action = EditorToolbarToolAction.RelatedNotes,
    ),
    EditorToolbarToolItem(
      icon = Lucide.MessageSquareText,
      label = "코멘트",
      action = EditorToolbarToolAction.Comment,
    ),
    EditorToolbarToolItem(
      icon = Lucide.SpellCheck,
      label = "맞춤법 검사",
      action = EditorToolbarToolAction.Spellcheck,
    ),
    EditorToolbarToolItem(
      icon = Lucide.Lightbulb,
      label = "AI 피드백",
      action = EditorToolbarToolAction.AiFeedback,
    ),
    EditorToolbarToolItem(
      icon = Lucide.History,
      label = "타임라인",
      action = EditorToolbarToolAction.Timeline,
    ),
  )

internal fun editorToolbarDebugToolItems(
  debugOverlays: EditorToolbarDebugOverlays
): List<EditorToolbarToolItem> {
  return buildList {
    add(
      EditorToolbarToolItem(
        icon = Lucide.PanelTop,
        label = debugOverlays.viewportVisible.debugToggleLabel("뷰포트 기준선"),
        action = EditorToolbarToolAction.DebugViewportOverlay,
        key = "debug-viewport-overlay",
      )
    )
    add(
      EditorToolbarToolItem(
        icon = Lucide.PanelBottom,
        label = debugOverlays.bodyVisible.debugToggleLabel("바디 영역"),
        action = EditorToolbarToolAction.DebugBodyOverlay,
        key = "debug-body-overlay",
      )
    )
    add(
      EditorToolbarToolItem(
        icon = Lucide.InspectionPanel,
        label = debugOverlays.surfaceVisible.debugToggleLabel("페이지 표면"),
        action = EditorToolbarToolAction.DebugSurfaceOverlay,
        key = "debug-surface-overlay",
      )
    )
    if (debugOverlays.inputLogAvailable) {
      add(
        EditorToolbarToolItem(
          icon = Lucide.Send,
          label = "입력 로그 보내기",
          action = EditorToolbarToolAction.SendInputLog,
          key = "debug-send-input-log",
        )
      )
    }
  }
}

private fun Boolean.debugToggleLabel(label: String): String = "$label ${if (this) "끄기" else "켜기"}"
