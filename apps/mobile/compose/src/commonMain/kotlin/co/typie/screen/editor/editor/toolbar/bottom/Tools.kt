package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarDebugOverlays
import co.typie.screen.editor.editor.toolbar.EditorToolbarToolAction
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelRadius
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
internal fun BottomToolbarTools(
  onAction: (EditorToolbarToolAction) -> Unit,
  debugOverlays: EditorToolbarDebugOverlays?,
  modifier: Modifier = Modifier,
) {
  val gridFogInsets = remember { PaddingValues(vertical = ToolPanelPadding) }

  LazyVerticalGrid(
    columns = GridCells.Fixed(2),
    modifier =
      modifier
        .fillMaxSize()
        .scrollFog(padding = gridFogInsets, color = AppTheme.colors.surfaceCanvas),
    contentPadding = PaddingValues(horizontal = ToolPanelPadding, vertical = ToolPanelPadding),
    horizontalArrangement = Arrangement.spacedBy(6.dp),
    verticalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    items(ToolItems, key = { it.key }) { item ->
      ToolTile(item = item, modifier = Modifier.fillMaxWidth(), onClick = { onAction(item.action) })
    }

    if (debugOverlays != null) {
      item(key = DebugSectionTitleKey, span = { GridItemSpan(maxLineSpan) }) {
        Text(
          text = "디버그",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.padding(start = 2.dp, top = 8.dp),
        )
      }
      items(debugToolItems(debugOverlays), key = { it.key }) { item ->
        ToolTile(
          item = item,
          modifier = Modifier.fillMaxWidth(),
          onClick = { onAction(item.action) },
        )
      }
    }
  }
}

private fun debugToolItems(debugOverlays: EditorToolbarDebugOverlays): List<ToolItem> {
  return listOf(
    ToolItem(
      icon = Lucide.PanelTop,
      label = debugOverlays.viewportVisible.debugToggleLabel("뷰포트 기준선"),
      action = EditorToolbarToolAction.DebugViewportOverlay,
      key = "debug-viewport-overlay",
    ),
    ToolItem(
      icon = Lucide.PanelBottom,
      label = debugOverlays.bodyVisible.debugToggleLabel("바디 영역"),
      action = EditorToolbarToolAction.DebugBodyOverlay,
      key = "debug-body-overlay",
    ),
    ToolItem(
      icon = Lucide.InspectionPanel,
      label = debugOverlays.surfaceVisible.debugToggleLabel("페이지 표면"),
      action = EditorToolbarToolAction.DebugSurfaceOverlay,
      key = "debug-surface-overlay",
    ),
  )
}

private fun Boolean.debugToggleLabel(label: String): String = "$label ${if (this) "끄기" else "켜기"}"

@Composable
private fun ToolTile(item: ToolItem, onClick: () -> Unit, modifier: Modifier = Modifier) {
  val shape = ToolTileShape

  InteractionScope {
    val interactionSource =
      LocalInteractionSource.current ?: remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()

    Box(
      modifier =
        modifier
          .height(ToolTileHeight)
          .focusProperties { canFocus = false }
          .clip(shape)
          .background(if (pressed) AppTheme.colors.surfaceInset else Color.Transparent, shape)
          .clickable(
            interactionSource = interactionSource,
            indication = null,
            role = Role.Button,
            onClickLabel = item.label,
            onClick = onClick,
          )
          .pressScale(ToolTilePressedScale)
          .padding(horizontal = 2.dp),
      contentAlignment = Alignment.Center,
    ) {
      Row(
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Icon(
          icon = item.icon,
          contentDescription = null,
          modifier = Modifier.size(ToolTileIconSize),
          tint = AppTheme.colors.textDefault,
        )
        Text(
          text = item.label,
          style = AppTheme.typography.body,
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          softWrap = false,
        )
      }
    }
  }
}

private data class ToolItem(
  val icon: IconData,
  val label: String,
  val action: EditorToolbarToolAction,
  val key: String = label,
)

private val ToolItems =
  listOf(
    ToolItem(icon = Lucide.StickyNote, label = "노트", action = EditorToolbarToolAction.RelatedNotes),
    ToolItem(
      icon = Lucide.MessageSquareText,
      label = "코멘트",
      action = EditorToolbarToolAction.Comment,
    ),
    ToolItem(
      icon = Lucide.SpellCheck,
      label = "맞춤법 검사",
      action = EditorToolbarToolAction.Spellcheck,
    ),
    ToolItem(
      icon = Lucide.Lightbulb,
      label = "AI 피드백",
      action = EditorToolbarToolAction.AiFeedback,
    ),
    ToolItem(icon = Lucide.History, label = "타임라인", action = EditorToolbarToolAction.Timeline),
  )

private const val DebugSectionTitleKey = "debug-section-title"
private val ToolPanelPadding = 16.dp
private val ToolTileShape =
  AppShapes.rounded(maxOf(AppShapes.sm, ToolbarBottomPanelRadius - ToolPanelPadding))
private val ToolTileHeight = 64.dp
private val ToolTileIconSize = 24.dp
private const val ToolTilePressedScale = 0.96f
