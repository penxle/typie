package co.typie.screen.editor.editor.entry

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.StablePosition
import co.typie.editor.ffi.StablePositionBinding
import co.typie.editor.ffi.StableSelection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorEntryStateStoreTest {
  @Test
  fun saves_entry_state_under_document_scoped_keys() {
    val rawEntries = mutableMapOf<String, String>()
    val store =
      EditorEntryStateStore(
        read = rawEntries::get,
        write = { documentId, value -> rawEntries[documentId] = value },
      )

    val first = StoredEditorEntryState(target = EditorEntryTarget.Title, updatedAt = 10L)
    val second =
      StoredEditorEntryState(
        target = EditorEntryTarget.Body,
        bodySelection = stableSelection(),
        updatedAt = 20L,
      )

    store.save(documentId = "doc-a", state = first)
    store.save(documentId = "doc-b", state = second)

    assertEquals(setOf("doc-a", "doc-b"), rawEntries.keys)
    assertEquals(first, store.load(documentId = "doc-a"))
    assertEquals(second, store.load(documentId = "doc-b"))
  }

  @Test
  fun returns_null_for_malformed_entry_state() {
    val rawEntries = mutableMapOf("doc-a" to "{")
    val store =
      EditorEntryStateStore(
        read = rawEntries::get,
        write = { documentId, value -> rawEntries[documentId] = value },
      )

    assertNull(store.load(documentId = "doc-a"))
  }
}

private fun stableSelection(): StableSelection {
  val position =
    StablePosition(
      chain = emptyList(),
      binding = StablePositionBinding.ContainerStart,
      affinity = Affinity.Downstream,
    )
  return StableSelection(anchor = position, head = position)
}
