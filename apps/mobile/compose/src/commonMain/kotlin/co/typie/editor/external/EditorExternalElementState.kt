package co.typie.editor.external

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateMapOf

@Stable
internal class EditorExternalElementState {
  val files = EditorExternalFileElementState()

  fun clear() {
    files.clear()
  }
}

@Stable
internal class EditorExternalFileElementState {
  val assets = mutableStateMapOf<String, EditorFileAsset>()
  val uploads = mutableStateMapOf<String, EditorFileUpload>()

  fun clear() {
    assets.clear()
    uploads.clear()
  }
}

internal data class EditorFileAsset(
  val id: String,
  val name: String,
  val url: String,
  val size: Long?,
)

internal class EditorFileUpload(val name: String, val size: Long?)

internal val LocalEditorExternalElementState =
  compositionLocalOf<EditorExternalElementState> { error("No EditorExternalElementState provided") }
