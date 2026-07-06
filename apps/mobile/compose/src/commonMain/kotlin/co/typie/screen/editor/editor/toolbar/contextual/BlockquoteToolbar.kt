package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.BlockquoteVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanel
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarNodeTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorBlockquoteToolbarPage(
  target: EditorToolbarNodeTarget<PlainNode.Blockquote>?
): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Blockquote,
    icon = Lucide.Quote,
    contentDescription = "인용구 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Lucide.Quote,
          contentDescription = "인용구 설정",
          onClick = {
            val currentTarget = target ?: return@EditorToolbarButton
            scope.toggleBottomPanel(
              EditorToolbarBottomPanel.BlockquoteVariants(
                target =
                  BlockquoteVariantPanelTarget.Existing(
                    nodeId = currentTarget.id,
                    currentVariant = currentTarget.node.variant,
                  )
              )
            )
          },
        )
        EditorToolbarButton(
          icon = Lucide.TextSelect,
          contentDescription = "일반 텍스트로",
          onClick = {
            val nodeId = target?.id ?: return@EditorToolbarButton
            scope.sendMessage(Message.Node(NodeOp.Unwrap(id = nodeId)))
          },
        )
      }
    },
  )
