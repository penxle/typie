package co.typie.screen.editor.editor.topbar

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarDebugOverlays
import co.typie.screen.editor.editor.toolbar.EditorToolbarToolAction
import co.typie.screen.editor.editor.toolbar.EditorToolbarToolItems
import co.typie.screen.editor.editor.toolbar.editorToolbarDebugToolItems
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.theme.AppTheme
import co.typie.ui.utils.ShortcutModifier
import co.typie.ui.utils.shortcutLabel

@Composable
internal fun EditorScreenTopBar(
  editing: Boolean,
  debugOverlays: EditorToolbarDebugOverlays?,
  documentButton: @Composable (Modifier) -> Unit,
  onToolAction: (EditorToolbarToolAction) -> Unit,
  onEnterReadingMode: suspend () -> Unit,
) {
  if (editing) {
    ProvideTopBar(
      backdropBlurEnabled = false,
      center = { documentButton(Modifier.fillMaxWidth()) },
      trailing = { EditorEditingTopBarTrailing(onEnterReadingMode) },
      trailingKey = EditingTopBarTrailingKey,
      scrollOffset = null,
    )
  } else {
    ProvideTopBar(
      backdropBlurEnabled = false,
      center = {
        documentButton(Modifier.fillMaxWidth().padding(end = ReadingTopBarExtraEndClearance))
      },
      trailing = { EditorReadingTopBarTrailing(debugOverlays, onToolAction) },
      trailingKey = ReadingTopBarTrailingKey,
      scrollOffset = null,
    )
  }
}

@Composable
private fun EditorEditingTopBarTrailing(onEnterReadingMode: suspend () -> Unit) {
  TopBarButton(
    icon = Lucide.Check,
    contentDescription = EnterReadingModeDescription,
    onClick = onEnterReadingMode,
    backgroundColor = AppTheme.colors.textDefault,
    contentColor = AppTheme.colors.surfaceDefault,
  )
}

@Composable
private fun EditorReadingTopBarTrailing(
  debugOverlays: EditorToolbarDebugOverlays?,
  onToolAction: (EditorToolbarToolAction) -> Unit,
) {
  Row(horizontalArrangement = Arrangement.spacedBy(ReadingTopBarActionGap)) {
    TopBarButton(
      icon = Lucide.Search,
      contentDescription = SearchDescription,
      shortcut = shortcutLabel("F", ShortcutModifier.Mod),
      onClick = { onToolAction(EditorToolbarToolAction.Search) },
    )
    PopoverMenu(
      anchor = { TopBarButton(icon = Lucide.Ellipsis, contentDescription = ToolsDescription) }
    ) {
      EditorToolbarToolItems.forEach { item ->
        item(icon = item.icon, label = item.label) { onToolAction(item.action) }
      }
      debugOverlays?.let { overlays ->
        divider()
        editorToolbarDebugToolItems(overlays).forEach { item ->
          item(icon = item.icon, label = item.label) { onToolAction(item.action) }
        }
      }
    }
  }
}

private val EditingTopBarTrailingKey = Any()
private val ReadingTopBarTrailingKey = Any()
private const val SearchDescription = "찾기 및 바꾸기"
private const val ToolsDescription = "도구"
private const val EnterReadingModeDescription = "읽기 모드로 전환"
private val ReadingTopBarActionGap = 8.dp
private val ReadingTopBarExtraEndClearance = TopBarDefaults.ButtonSize + ReadingTopBarActionGap
