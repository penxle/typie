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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.ext.isSingleSlotRange
import co.typie.editor.ffi.BlockOp
import co.typie.editor.ffi.Break
import co.typie.editor.ffi.Fragment
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.ListKind
import co.typie.editor.ffi.ListOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Selection
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.BlockquoteVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanel
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelRadius
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
internal fun BottomToolbarNodes(
  onEditorInputRequest: () -> Unit,
  onBottomPanelRequest: (EditorToolbarBottomPanel) -> Unit,
  modifier: Modifier = Modifier,
) {
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val editor = runtime.editor
  val showPageBreak = editor?.rootAttrs?.layoutMode is LayoutMode.Paginated
  val hasUnitSelection =
    isEditorToolbarUnitSelection(
      selection = editor?.selection,
      hasSelectedBlock = editor?.blockState?.nodes?.isNotEmpty() == true,
    )
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
    items(
      editorToolbarNodeInsertItems(
        showPageBreak = showPageBreak,
        hasUnitSelection = hasUnitSelection,
      ),
      key = { it.label },
    ) { item ->
      NodeInsertTile(
        item = item,
        modifier = Modifier.fillMaxWidth(),
        onClick = {
          when (val action = item.action) {
            is EditorToolbarNodeInsertAction.OpenPanel -> onBottomPanelRequest(action.panel)
            is EditorToolbarNodeInsertAction.SendMessage -> {
              val session = runtime.session ?: return@NodeInsertTile
              session.submit { currentEditor, context ->
                currentEditor.scope.launch(context) {
                  currentEditor.awaitWithBringIntoView(bringIntoViewRequests) {
                    enqueue(action.message)
                    beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
                  }
                }
              }
              onEditorInputRequest()
            }
          }
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
  val action: EditorToolbarNodeInsertAction,
)

internal sealed interface EditorToolbarNodeInsertAction {
  data class SendMessage(val message: Message) : EditorToolbarNodeInsertAction

  data class OpenPanel(val panel: EditorToolbarBottomPanel) : EditorToolbarNodeInsertAction
}

internal fun editorToolbarNodeInsertItems(
  showPageBreak: Boolean,
  hasUnitSelection: Boolean,
): List<EditorToolbarNodeInsertItem> =
  listOf(
      EditorToolbarNodeInsertItem(
        icon = Lucide.Image,
        label = "이미지",
        action =
          EditorToolbarNodeInsertAction.SendMessage(fragmentInsertion(PlainNode.Image(id = null))),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Paperclip,
        label = "파일",
        action =
          EditorToolbarNodeInsertAction.SendMessage(fragmentInsertion(PlainNode.File(id = null))),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.FileUp,
        label = "임베드",
        action =
          EditorToolbarNodeInsertAction.SendMessage(fragmentInsertion(PlainNode.Embed(id = null))),
      ),
      EditorToolbarNodeInsertItem(
        icon = Typie.HorizontalRule,
        label = "구분선",
        action =
          EditorToolbarNodeInsertAction.OpenPanel(
            EditorToolbarBottomPanel.HorizontalRuleVariants(
              target = HorizontalRuleVariantPanelTarget.Insertion
            )
          ),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Quote,
        label = "인용구",
        action =
          EditorToolbarNodeInsertAction.OpenPanel(
            EditorToolbarBottomPanel.BlockquoteVariants(
              target = BlockquoteVariantPanelTarget.Selection
            )
          ),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.GalleryVerticalEnd,
        label = "강조",
        action = EditorToolbarNodeInsertAction.SendMessage(Message.Block(BlockOp.ToggleCallout)),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.ChevronsDownUp,
        label = "접기",
        action = EditorToolbarNodeInsertAction.SendMessage(Message.Block(BlockOp.WrapFold)),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.Table,
        label = "표",
        action = EditorToolbarNodeInsertAction.OpenPanel(EditorToolbarBottomPanel.TableSizeSelector),
      ),
      EditorToolbarNodeInsertItem(
        icon = Lucide.List,
        label = "목록",
        action =
          EditorToolbarNodeInsertAction.SendMessage(
            Message.List(ListOp.ToggleKind(ListKind.Bullet))
          ),
      ),
      if (showPageBreak) {
        EditorToolbarNodeInsertItem(
          icon = Lucide.FilePlus,
          label = "페이지 나누기",
          action =
            EditorToolbarNodeInsertAction.SendMessage(
              Message.Insertion(InsertionOp.Break(Break.Page))
            ),
        )
      } else {
        null
      },
      EditorToolbarNodeInsertItem(
        icon = if (hasUnitSelection) Lucide.CornerLeftUp else Lucide.CornerDownLeft,
        label = if (hasUnitSelection) "위에 문단 넣기" else "문단 내 줄바꿈",
        action =
          EditorToolbarNodeInsertAction.SendMessage(
            Message.Key(KeyEvent(Key.Enter, InputModifiers(shift = true)))
          ),
      ),
    )
    .filterNotNull()

private fun fragmentInsertion(node: PlainNode): Message.Insertion =
  Message.Insertion(InsertionOp.Fragment(Fragment(node = node)))

internal fun isEditorToolbarUnitSelection(
  selection: Selection?,
  hasSelectedBlock: Boolean,
): Boolean = selection.isSingleSlotRange() && hasSelectedBlock

private val NodeInsertPanelPadding = 16.dp
private val NodeInsertTileShape =
  AppShapes.rounded(maxOf(AppShapes.sm, ToolbarBottomPanelRadius - NodeInsertPanelPadding))
private val NodeInsertGridMinCellWidth = 78.dp
private val NodeInsertTileHeight = 64.dp
private const val NodeInsertTilePressedScale = 0.96f
