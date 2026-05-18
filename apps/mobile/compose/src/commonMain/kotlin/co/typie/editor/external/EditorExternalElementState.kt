package co.typie.editor.external

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateMapOf

@Stable
internal class EditorExternalElementState {
  val images = EditorExternalImageElementState()
  val files = EditorExternalFileElementState()

  fun clear() {
    images.clear()
    files.clear()
  }
}

@Stable
internal class EditorExternalImageElementState {
  val assets = mutableStateMapOf<String, EditorImageAsset>()
  val uploads = mutableStateMapOf<String, EditorImageUpload>()

  fun clear() {
    assets.clear()
    uploads.clear()
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

internal data class EditorImageAsset(
  val id: String,
  val url: String,
  val width: Int,
  val height: Int,
  val ratio: Double,
  val placeholder: String?,
)

internal class EditorImageUpload(
  val bytes: ByteArray,
  val name: String,
  val width: Int,
  val height: Int,
) {
  val ratio: Double
    get() = width.toDouble() / height.toDouble()
}

internal class EditorFileUpload(val name: String, val size: Long?)

internal val LocalEditorExternalElementState =
  compositionLocalOf<EditorExternalElementState> { error("No EditorExternalElementState provided") }
