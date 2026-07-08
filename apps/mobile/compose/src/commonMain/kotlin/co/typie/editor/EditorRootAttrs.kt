package co.typie.editor

import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode

// Wire form of editor_crdt Dot::ROOT ("{base62(actor)}_{base62(clock)}").
// Pinned in crates/editor-crdt/src/dot.rs (root_string_form_is_pinned_for_ffi_clients).
internal const val EditorRootId = "0_AzL8n0Y58m8"

internal val DefaultRootPaginatedLayout =
  LayoutMode.Paginated(
    pageWidth = 794,
    pageHeight = 1123,
    pageMarginTop = 94,
    pageMarginBottom = 94,
    pageMarginLeft = 94,
    pageMarginRight = 94,
  )

internal fun EditorScope.enqueueRootLayoutMode(layoutMode: LayoutMode) {
  enqueue(
    Message.Node(
      NodeOp.SetAttrs(id = EditorRootId, attrs = PlainNode.Root(layoutMode = layoutMode))
    )
  )
}

internal fun EditorScope.enqueueRootModifier(modifier: EditorModifier) {
  enqueue(Message.Modifier(ModifierOp.SetOnNode(id = EditorRootId, modifier = modifier)))
}
