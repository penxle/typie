package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanel
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarNodeTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget

internal fun editorHorizontalRuleToolbarPage(
  target: EditorToolbarNodeTarget<PlainNode.HorizontalRule>?
): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.HorizontalRule,
    icon = Typie.HorizontalRule,
    contentDescription = "구분선 툴바",
    ownerNodeId = target?.id,
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Typie.HorizontalRule,
          contentDescription = "구분선 설정",
          onClick = {
            val currentTarget = target ?: return@EditorToolbarButton
            scope.toggleBottomPanel(
              EditorToolbarBottomPanel.HorizontalRuleVariants(
                target =
                  HorizontalRuleVariantPanelTarget.Existing(
                    nodeId = currentTarget.id,
                    currentVariant = currentTarget.node.variant,
                  )
              )
            )
          },
        )
        EditorToolbarButton(
          icon = Lucide.Trash2,
          contentDescription = "구분선 삭제",
          onClick = {
            val nodeId = target?.id ?: return@EditorToolbarButton
            scope.sendMessage(Message.Node(NodeOp.Delete(id = nodeId)))
          },
        )
      }
    },
  )
