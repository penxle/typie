package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.editor.ffi.Alignment as FfiAlignment
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.screen.editor.editor.toolbar.EditorToolbarLabelButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarTableTarget
import co.typie.screen.editor.editor.toolbar.ToolbarFixedActionWidth
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarPageEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding

@Composable
internal fun TableAlignmentSecondaryToolbar(
  target: EditorToolbarTableTarget,
  onClose: () -> Unit,
  sendMessage: (Message) -> Unit,
  modifier: Modifier = Modifier,
) {
  ToolbarSecondarySurface(
    onClose = onClose,
    closeContentDescription = "표 정렬 닫기",
    modifier = modifier,
  ) {
    Row(
      modifier =
        Modifier.fillMaxSize()
          .padding(
            start = ToolbarFixedActionWidth,
            top = ToolbarPageVerticalPadding,
            end = ToolbarPageEndPadding,
            bottom = ToolbarPageVerticalPadding,
          ),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
    ) {
      listOf(FfiAlignment.Left, FfiAlignment.Center, FfiAlignment.Right).forEach { alignment ->
        EditorToolbarLabelButton(
          text = toolbarAlignmentLabel(alignment),
          contentDescription = "표 ${toolbarAlignmentLabel(alignment)} 정렬",
          onClick = {
            tableAlignmentMessageOrNull(target, alignment)?.let(sendMessage)
            onClose()
          },
          selected = target.align == alignment,
          autoBringIntoView = target.align == alignment,
          subtle = true,
        )
      }
    }
  }
}

internal fun tableAlignmentMessageOrNull(
  target: EditorToolbarTableTarget,
  alignment: FfiAlignment,
): Message? =
  if (target.align == alignment) {
    null
  } else {
    Message.Modifier(
      ModifierOp.SetOnNode(id = target.id, modifier = EditorModifier.Alignment(alignment))
    )
  }
