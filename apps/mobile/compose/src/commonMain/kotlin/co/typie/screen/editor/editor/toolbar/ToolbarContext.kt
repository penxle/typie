package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ext.isSingleSlotRange
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.BackgroundColorValue
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.TableOverlay
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
  val tableTarget: EditorToolbarTableTarget? = null,
)

internal data class EditorToolbarNodeTarget<out T : PlainNode>(val id: String, val node: T)

internal data class EditorToolbarTableTarget(
  val id: String,
  val node: PlainNode.Table,
  val selected: Boolean,
  val align: Alignment,
  val cellBackgroundCurrentValue: String?,
)

internal data class EditorToolbarAutoTargetKey(
  val pageKey: EditorToolbarPageKey,
  val selectedNodeId: String?,
)

internal enum class EditorToolbarListMode {
  Bullet,
  Ordered,
}

internal fun resolveEditorToolbarContext(state: EditorState): EditorToolbarContext {
  val selection = state.selection
  val blockState = state.blockState
  val selectionCollapsed = selection.isCollapsed()
  val nodes = blockState?.nodes.orEmpty()
  val ancestors = blockState?.ancestors.orEmpty()
  val intersectingNodes = blockState?.intersectingNodes.orEmpty()
  val hasTextPage = state.modifierState?.hasInlineTextModifier() == true
  val selectedBlock =
    if (selection.isSingleSlotRange()) {
      nodes.firstOrNull { it.node.selectedToolbarPageKey() != null }
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
  var blockquoteTarget =
    (selectedBlock?.node as? PlainNode.Blockquote)?.let { node ->
      EditorToolbarNodeTarget(id = selectedBlock.id, node = node)
    }
  var calloutTarget =
    (selectedBlock?.node as? PlainNode.Callout)?.let { node ->
      EditorToolbarNodeTarget(id = selectedBlock.id, node = node)
    }
  var foldTargetId = selectedBlock?.id?.takeIf { selectedBlock.node == PlainNode.Fold }
  var tableTarget =
    (selectedBlock?.node as? PlainNode.Table)?.let { node ->
      val overlay = state.tableOverlays.tableOverlay(selectedBlock.id)
      EditorToolbarTableTarget(
        id = selectedBlock.id,
        node = node,
        selected = true,
        align = overlay?.align ?: Alignment.Left,
        cellBackgroundCurrentValue =
          state.modifierState?.cellBackgroundColor.tableCellBackgroundCurrentValue(),
      )
    }
  val ancestorIds = ancestors.mapTo(mutableSetOf()) { it.id }

  ancestors.forEach { block ->
    val blockListMode = block.node.toolbarListMode()
    if (blockListMode != null) {
      if (listMode == null) {
        listMode = blockListMode
      }
      addPage(EditorToolbarPageKey.List)
    } else {
      when (block.node) {
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
          if (tableTarget == null) {
            val overlay = state.tableOverlays.tableOverlay(block.id)
            tableTarget =
              EditorToolbarTableTarget(
                id = block.id,
                node = block.node,
                selected = false,
                align = overlay?.align ?: Alignment.Left,
                cellBackgroundCurrentValue =
                  state.modifierState?.cellBackgroundColor.tableCellBackgroundCurrentValue(),
              )
          }
          addPage(EditorToolbarPageKey.Table)
        }
        else -> Unit
      }
    }
  }
  if (
    !selectionCollapsed &&
      intersectingNodes.any { block ->
        when (block.node) {
          PlainNode.BulletList,
          PlainNode.OrderedList,
          PlainNode.ListItem -> true
          else -> false
        }
      }
  ) {
    addPage(EditorToolbarPageKey.List)
    val mixedListMode =
      listMode != null &&
        intersectingNodes
          .asSequence()
          .filterNot { it.id in ancestorIds }
          .mapNotNull { it.node.toolbarListMode() }
          .any { it != listMode }
    if (mixedListMode) {
      listMode = null
    }
    if (listMode == null && !mixedListMode) {
      listMode = nodes.mapNotNull { it.node.toolbarListMode() }.distinct().singleOrNull()
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
    tableTarget = tableTarget,
  )
}

private fun List<TableOverlay>.tableOverlay(tableId: String): TableOverlay? = firstOrNull {
  it.tableId == tableId
}

private fun Tri<BackgroundColorValue>?.tableCellBackgroundCurrentValue(): String? =
  when (this) {
    is Tri.Uniform -> value.value
    Tri.Absent -> "none"
    Tri.Mixed,
    null -> null
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

private fun PlainNode.toolbarListMode(): EditorToolbarListMode? =
  when (this) {
    PlainNode.BulletList -> EditorToolbarListMode.Bullet
    PlainNode.OrderedList -> EditorToolbarListMode.Ordered
    else -> null
  }

private fun PlainNode.selectedToolbarPageKey(): EditorToolbarPageKey? =
  when (this) {
    is PlainNode.Image -> EditorToolbarPageKey.Image
    is PlainNode.File -> EditorToolbarPageKey.File
    is PlainNode.Embed -> EditorToolbarPageKey.Embed
    is PlainNode.Archived -> EditorToolbarPageKey.Archived
    is PlainNode.HorizontalRule -> EditorToolbarPageKey.HorizontalRule
    is PlainNode.Blockquote -> EditorToolbarPageKey.Blockquote
    is PlainNode.Callout -> EditorToolbarPageKey.Callout
    PlainNode.Fold -> EditorToolbarPageKey.Fold
    is PlainNode.Table -> EditorToolbarPageKey.Table
    else -> null
  }
