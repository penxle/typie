package co.typie.screen.document.document

import co.typie.editor.ffi.CharacterCounts

internal object DocumentCharacterCountSnapshots {
  private val snapshots = mutableMapOf<String, CharacterCounts>()

  fun put(entityId: String, counts: CharacterCounts?) {
    if (counts == null) {
      remove(entityId)
    } else {
      snapshots[entityId] = counts
    }
  }

  fun take(entityId: String): CharacterCounts? = snapshots.remove(entityId)

  fun remove(entityId: String) {
    snapshots.remove(entityId)
  }
}
