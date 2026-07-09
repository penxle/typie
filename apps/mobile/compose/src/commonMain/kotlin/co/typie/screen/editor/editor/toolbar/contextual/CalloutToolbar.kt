package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.CalloutVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarNodeTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorCalloutToolbarPage(
  target: EditorToolbarNodeTarget<PlainNode.Callout>?
): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Callout,
    icon = Lucide.GalleryVerticalEnd,
    contentDescription = "강조 툴바",
    ownerNodeId = target?.id,
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = target?.node?.variant?.icon ?: Lucide.GalleryVerticalEnd,
          contentDescription = "강조 종류",
          onClick = {
            val nodeId = target?.id ?: return@EditorToolbarButton
            scope.sendMessage(Message.Node(NodeOp.CycleCalloutVariant(id = nodeId)))
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

private val CalloutVariant.icon
  get() =
    when (this) {
      CalloutVariant.Info -> Lucide.Info
      CalloutVariant.Success -> Lucide.CircleCheck
      CalloutVariant.Warning -> Lucide.CircleAlert
      CalloutVariant.Danger -> Lucide.TriangleAlert
    }
