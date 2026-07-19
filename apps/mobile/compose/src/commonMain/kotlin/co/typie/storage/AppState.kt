package co.typie.storage

import co.typie.platform.PlatformModule
import eu.anifantakis.lib.ksafe.KSafeWriteMode

object AppState {
  private const val EditorEntryStatePrefix = "editor_entry_state_v2:"

  fun getSerializedEditorEntryState(documentId: String): String? =
    PlatformModule.ksafeState.getDirect<String?>(editorEntryStateKey(documentId), null)

  fun setSerializedEditorEntryState(documentId: String, value: String) {
    PlatformModule.ksafeState.putDirect(
      key = editorEntryStateKey(documentId),
      value = value,
      mode = KSafeWriteMode.Plain,
    )
  }

  private fun editorEntryStateKey(documentId: String): String = "$EditorEntryStatePrefix$documentId"
}
