package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.Break
import co.typie.editor.ffi.Fragment
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlainNode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelRadius
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
internal fun BottomToolbarNodes(onEditorInputRequest: () -> Unit, modifier: Modifier = Modifier) {
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val scope = rememberCoroutineScope()
  val editor = runtime.editor
  val showPageBreak = editor?.rootAttrs?.layoutMode is LayoutMode.Paginated
  val gridFogInsets = remember { PaddingValues(vertical = NodeInsertPanelPadding) }

  LazyVerticalGrid(
    columns = GridCells.Adaptive(minSize = NodeInsertGridMinCellWidth),
    modifier =
      modifier
        .fillMaxSize()
        .scrollFog(padding = gridFogInsets, color = AppTheme.colors.surfaceCanvas),
    contentPadding =
      PaddingValues(horizontal = NodeInsertPanelPadding, vertical = NodeInsertPanelPadding),
    horizontalArrangement = Arrangement.spacedBy(6.dp),
    verticalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    items(editorToolbarNodeInsertItems(showPageBreak = showPageBreak), key = { it.label }) { item ->
      NodeInsertTile(
        item = item,
        modifier = Modifier.fillMaxWidth(),
        onClick = {
          val currentEditor = runtime.editor ?: return@NodeInsertTile
          scope.launch {
            currentEditor.awaitWithBringIntoView(bringIntoViewRequests) {
              enqueue(item.message)
              beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
            }
          }
          onEditorInputRequest()
        },
      )
    }
  }
}

@Composable
private fun NodeInsertTile(
  item: EditorToolbarNodeInsertItem,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val shape = NodeInsertTileShape

  InteractionScope {
    val interactionSource =
      LocalInteractionSource.current ?: remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()

    Box(
      modifier =
        modifier
          .height(NodeInsertTileHeight)
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
          .pressScale(NodeInsertTilePressedScale)
          .padding(horizontal = 2.dp),
      contentAlignment = Alignment.Center,
    ) {
      Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(5.dp),
      ) {
        Icon(
          icon = item.icon,
          contentDescription = null,
          modifier = Modifier.size(22.dp),
          tint = AppTheme.colors.textDefault,
        )
        Text(
          text = item.label,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textDefault,
          textAlign = TextAlign.Center,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          softWrap = false,
        )
      }
    }
  }
}

internal data class EditorToolbarNodeInsertItem(
  val icon: IconData,
  val label: String,
  val message: Message.Insertion,
)

internal fun editorToolbarNodeInsertItems(
  showPageBreak: Boolean
): List<EditorToolbarNodeInsertItem> =
  listOf(
      EditorToolbarNodeInsertItem(
        icon = Lucide.Image,
        label = "이미지",
        message = fragmentInsertion(PlainNode.Image(id = null)),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Paperclip,
        label = "파일",
        message = fragmentInsertion(PlainNode.File(id = null)),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.FileUp,
        label = "임베드",
        message = fragmentInsertion(PlainNode.Embed(id = null)),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Scissors,
        label = "구분선",
        message = fragmentInsertion(PlainNode.HorizontalRule()),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Quote,
        label = "인용구",
        message = fragmentInsertion(PlainNode.Blockquote()),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.GalleryVerticalEnd,
        label = "강조",
        message = fragmentInsertion(PlainNode.Callout()),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.ChevronsDownUp,
        label = "접기",
        message = fragmentInsertion(PlainNode.Fold),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Table,
        label = "표",
        message = fragmentInsertion(PlainNode.Table()),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.List,
        label = "목록",
        message = fragmentInsertion(PlainNode.BulletList),
      ),
      if (showPageBreak) {
        EditorToolbarNodeInsertItem(
          icon = Lucide.FilePlus,
          label = "페이지 나누기",
          message = Message.Insertion(InsertionOp.Break(Break.Page)),
        )
      } else {
        null
      },
      EditorToolbarNodeInsertItem(
        icon = Lucide.CornerDownLeft,
        label = "문단 내 줄바꿈",
        message = Message.Insertion(InsertionOp.Break(Break.Line)),
      ),
    )
    .filterNotNull()

private fun fragmentInsertion(node: PlainNode): Message.Insertion =
  Message.Insertion(InsertionOp.Fragment(Fragment(node = node)))

private val NodeInsertPanelPadding = 16.dp
private val NodeInsertTileShape =
  AppShapes.rounded(maxOf(AppShapes.sm, ToolbarBottomPanelRadius - NodeInsertPanelPadding))
private val NodeInsertGridMinCellWidth = 78.dp
private val NodeInsertTileHeight = 64.dp
private const val NodeInsertTilePressedScale = 0.96f
