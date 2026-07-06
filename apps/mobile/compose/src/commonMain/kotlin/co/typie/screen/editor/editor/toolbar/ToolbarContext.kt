package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ext.isSingleSlotRange
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Tri

internal data class EditorToolbarContext(
  val pageKeys: List<EditorToolbarPageKey>,
  val autoTargetPageKey: EditorToolbarPageKey?,
  val autoTargetKey: EditorToolbarAutoTargetKey? = null,
  val selectedNode: PlainNode? = null,
  val selectedNodeId: String? = null,
  val horizontalRuleTarget: EditorToolbarNodeTarget<PlainNode.HorizontalRule>? = null,
  val blockquoteTarget: EditorToolbarNodeTarget<PlainNode.Blockquote>? = null,
  val calloutTarget: EditorToolbarNodeTarget<PlainNode.Callout>? = null,
  val foldTargetId: String? = null,
  val listMode: EditorToolbarListMode? = null,
  val tableMode: EditorToolbarTableMode? = null,
)

internal data class EditorToolbarNodeTarget<out T : PlainNode>(val id: String, val node: T)

internal data class EditorToolbarAutoTargetKey(
  val pageKey: EditorToolbarPageKey,
  val selectedNodeId: String?,
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
  val horizontalRuleTarget =
    (selectedBlock?.node as? PlainNode.HorizontalRule)?.let { node ->
      EditorToolbarNodeTarget(id = selectedBlock.id, node = node)
    }
  var blockquoteTarget: EditorToolbarNodeTarget<PlainNode.Blockquote>? = null
  var calloutTarget: EditorToolbarNodeTarget<PlainNode.Callout>? = null
  var foldTargetId: String? = null
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
      is PlainNode.Blockquote -> {
        if (blockquoteTarget == null) {
          blockquoteTarget = EditorToolbarNodeTarget(id = block.id, node = block.node)
        }
        addPage(EditorToolbarPageKey.Blockquote)
      }
      is PlainNode.Callout -> {
        if (calloutTarget == null) {
          calloutTarget = EditorToolbarNodeTarget(id = block.id, node = block.node)
        }
        addPage(EditorToolbarPageKey.Callout)
      }
      PlainNode.Fold -> {
        if (foldTargetId == null) {
          foldTargetId = block.id
        }
        addPage(EditorToolbarPageKey.Fold)
      }
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
    autoTargetKey =
      selectedPageKey?.let { pageKey ->
        EditorToolbarAutoTargetKey(pageKey = pageKey, selectedNodeId = selectedBlock.id)
      },
    selectedNode = selectedBlock?.node,
    selectedNodeId = selectedBlock?.id,
    horizontalRuleTarget = horizontalRuleTarget,
    blockquoteTarget = blockquoteTarget,
    calloutTarget = calloutTarget,
    foldTargetId = foldTargetId,
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
