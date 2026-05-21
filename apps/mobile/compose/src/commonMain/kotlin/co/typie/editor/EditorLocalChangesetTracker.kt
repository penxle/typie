package co.typie.editor

internal data class EditorLocalChangesets(
  val baseHeads: ByteArray,
  val currentHeads: ByteArray,
  val changesets: ByteArray,
)

internal class EditorLocalChangesetTracker {
  private var syncedHeads: ByteArray? = null

  suspend fun markSynced(editor: Editor) {
    syncedHeads = editor.currentHeads()
  }

  suspend fun collect(editor: Editor, block: EditorScope.() -> Unit): ByteArray {
    val localChangesets = editor.collectLocalChangesets(baseHeads = syncedHeads, block = block)
    syncedHeads =
      if (localChangesets.changesets.isEmpty()) {
        localChangesets.currentHeads
      } else {
        localChangesets.baseHeads
      }
    return localChangesets.changesets
  }

  fun markSynced(heads: ByteArray) {
    syncedHeads = heads
  }
}
