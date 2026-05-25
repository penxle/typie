package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ext.isSingleSlotRange
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Tri

internal data class EditorToolbarContext(
  val pageKeys: List<EditorToolbarPageKey>,
  val autoTargetPageKey: EditorToolbarPageKey?,
  val selectedNode: PlainNode? = null,
  val selectedNodeId: String? = null,
  val listMode: EditorToolbarListMode? = null,
  val tableMode: EditorToolbarTableMode? = null,
)

internal enum class EditorToolbarListMode {
  Bullet,
  Ordered,
}

internal enum class EditorToolbarTableMode {
  Selected,
  InTable,
}

internal fun resolveEditorToolbarContext(state: EditorState): EditorToolbarContext {
  val selection = state.selection
  val blockState = state.blockState
  val hasTextPage = state.modifierState?.hasInlineTextModifier() == true
  val selectedBlock =
    if (selection.isSingleSlotRange()) {
      blockState?.nodes?.firstOrNull { it.node.selectedToolbarPageKey() != null }
    } else {
      null
    }
  val selectedPageKey = selectedBlock?.node?.selectedToolbarPageKey()

  val pageKeys = mutableListOf(EditorToolbarPageKey.Main)
  fun addPage(key: EditorToolbarPageKey) {
    if (!pageKeys.contains(key)) {
      pageKeys += key
    }
  }

  if (hasTextPage) {
    addPage(EditorToolbarPageKey.Text)
  }
  if (selectedPageKey != null) {
    addPage(selectedPageKey)
  }

  var listMode: EditorToolbarListMode? = null
  var tableMode: EditorToolbarTableMode? =
    if (selectedPageKey == EditorToolbarPageKey.Table) EditorToolbarTableMode.Selected else null

  blockState?.ancestors.orEmpty().forEach { block ->
    when (block.node) {
      PlainNode.BulletList -> {
        listMode = EditorToolbarListMode.Bullet
        addPage(EditorToolbarPageKey.List)
      }
      PlainNode.OrderedList -> {
        listMode = EditorToolbarListMode.Ordered
        addPage(EditorToolbarPageKey.List)
      }
      is PlainNode.Blockquote -> addPage(EditorToolbarPageKey.Blockquote)
      is PlainNode.Callout -> addPage(EditorToolbarPageKey.Callout)
      PlainNode.Fold -> addPage(EditorToolbarPageKey.Fold)
      is PlainNode.Table -> {
        if (tableMode == null) {
          tableMode = EditorToolbarTableMode.InTable
        }
        addPage(EditorToolbarPageKey.Table)
      }
      else -> Unit
    }
  }

  return EditorToolbarContext(
    pageKeys = pageKeys,
    autoTargetPageKey = selectedPageKey,
    selectedNode = selectedBlock?.node,
    selectedNodeId = selectedBlock?.id,
    listMode = listMode,
    tableMode = tableMode,
  )
}

private fun ModifierState.hasInlineTextModifier(): Boolean =
  bold.isPresent() ||
    italic.isPresent() ||
    underline.isPresent() ||
    strikethrough.isPresent() ||
    fontSize.isPresent() ||
    fontFamily.isPresent() ||
    fontWeight.isPresent() ||
    textColor.isPresent() ||
    backgroundColor.isPresent() ||
    letterSpacing.isPresent() ||
    link.isPresent() ||
    ruby.isPresent()

private fun Tri<*>.isPresent(): Boolean = this !is Tri.Absent

private fun PlainNode.selectedToolbarPageKey(): EditorToolbarPageKey? =
  when (this) {
    is PlainNode.Image -> EditorToolbarPageKey.Image
    is PlainNode.File -> EditorToolbarPageKey.File
    is PlainNode.Embed -> EditorToolbarPageKey.Embed
    is PlainNode.Archived -> EditorToolbarPageKey.Archived
    is PlainNode.HorizontalRule -> EditorToolbarPageKey.HorizontalRule
    is PlainNode.Table -> EditorToolbarPageKey.Table
    else -> null
  }
