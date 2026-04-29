package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.Node
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Tri
import kotlin.math.abs

internal data class EditorToolbarContext(
  val pageKeys: List<EditorToolbarPageKey>,
  val autoTargetPageKey: EditorToolbarPageKey?,
  val selectedNode: Node? = null,
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
    if (selection?.isSingleBlockRange() == true) {
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
      Node.BulletList -> {
        listMode = EditorToolbarListMode.Bullet
        addPage(EditorToolbarPageKey.List)
      }
      Node.OrderedList -> {
        listMode = EditorToolbarListMode.Ordered
        addPage(EditorToolbarPageKey.List)
      }
      is Node.Blockquote -> addPage(EditorToolbarPageKey.Blockquote)
      is Node.Callout -> addPage(EditorToolbarPageKey.Callout)
      Node.Fold -> addPage(EditorToolbarPageKey.Fold)
      is Node.Table -> {
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

private fun Selection.isSingleBlockRange(): Boolean =
  anchor.nodeId == head.nodeId && abs(anchor.offset - head.offset) == 1

private fun Node.selectedToolbarPageKey(): EditorToolbarPageKey? =
  when (this) {
    is Node.Image -> EditorToolbarPageKey.Image
    is Node.File -> EditorToolbarPageKey.File
    is Node.Embed -> EditorToolbarPageKey.Embed
    is Node.Archived -> EditorToolbarPageKey.Archived
    is Node.HorizontalRule -> EditorToolbarPageKey.HorizontalRule
    is Node.Table -> EditorToolbarPageKey.Table
    else -> null
  }
