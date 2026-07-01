package co.typie.screen.editor.editor.entry

import co.typie.editor.ffi.StableSelection
import co.typie.serialization.json
import co.typie.storage.AppState
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString

@Serializable
internal enum class EditorEntryTarget {
  Title,
  Subtitle,
  Body,
}

@Serializable
internal data class StoredEditorEntryState(
  val target: EditorEntryTarget,
  val bodySelection: StableSelection? = null,
  val updatedAt: Long,
)

internal class EditorEntryStateStore(
  private val read: (documentId: String) -> String? = AppState::getSerializedEditorEntryState,
  private val write: (documentId: String, value: String) -> Unit =
    AppState::setSerializedEditorEntryState,
) {
  fun load(documentId: String): StoredEditorEntryState? {
    val raw = read(documentId) ?: return null
    return try {
      json.decodeFromString<StoredEditorEntryState>(raw)
    } catch (_: Exception) {
      null
    }
  }

  fun save(documentId: String, state: StoredEditorEntryState) {
    write(documentId, json.encodeToString(state))
  }
}
