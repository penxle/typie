package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorFoldToolbarPage(targetId: String?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Fold,
    icon = Lucide.ChevronsDownUp,
    contentDescription = "접기 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Lucide.TextSelect,
          contentDescription = "일반 텍스트로",
          onClick = {
            val nodeId = targetId ?: return@EditorToolbarButton
            scope.sendMessage(Message.Node(NodeOp.Unwrap(id = nodeId)))
          },
        )
      }
    },
  )
