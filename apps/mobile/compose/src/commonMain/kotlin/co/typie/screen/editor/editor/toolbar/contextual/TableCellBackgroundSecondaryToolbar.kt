package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.editor.EditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.TableOp
import co.typie.screen.editor.editor.toolbar.EditorToolbarTableTarget
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarPageEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentStartInset

@Composable
internal fun TableCellBackgroundSecondaryToolbar(
  target: EditorToolbarTableTarget,
  onClose: () -> Unit,
  sendMessage: (Message) -> Unit,
  modifier: Modifier = Modifier,
) {
  val scrollState = rememberScrollState()
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }

  ToolbarSecondarySurface(
    onClose = onClose,
    closeContentDescription = "셀 배경색 닫기",
    modifier = modifier,
  ) {
    Row(
      modifier =
        Modifier.fillMaxSize()
          .horizontalScroll(scrollState)
          .padding(
            start = ToolbarSecondaryContentStartInset,
            top = ToolbarPageVerticalPadding,
            end = ToolbarPageEndPadding,
            bottom = ToolbarPageVerticalPadding,
          ),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
    ) {
      TextBackgroundColorOptions(
        currentValue = target.cellBackgroundCurrentValue,
        editorTheme = editorTheme,
        onSelect = { value -> sendMessage(tableCellBackgroundMessage(target.id, value)) },
      )
    }
  }
}

internal fun tableCellBackgroundMessage(tableId: String, value: String): Message =
  Message.Node(
    NodeOp.Table(
      id = tableId,
      op = TableOp.SetCellBackgroundColor(color = value.takeUnless { it == "none" }),
    )
  )
