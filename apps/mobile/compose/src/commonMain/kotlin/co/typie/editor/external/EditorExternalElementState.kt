package co.typie.editor.external

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateMapOf

@Stable
internal class EditorExternalElementState {
  val images = EditorExternalImageElementState()
  val files = EditorExternalFileElementState()
  val embeds = EditorExternalEmbedElementState()

  fun clear() {
    images.clear()
    files.clear()
    embeds.clear()
  }
}

@Stable
internal class EditorExternalImageElementState {
  val assets = mutableStateMapOf<String, EditorImageAsset>()
  val uploads = mutableStateMapOf<String, EditorImageUpload>()
  val resizeDraftProportions = mutableStateMapOf<String, Float>()

  fun clearResizeState(nodeId: String) {
    resizeDraftProportions.remove(nodeId)
  }

  fun clear() {
    assets.clear()
    uploads.clear()
    resizeDraftProportions.clear()
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

@Stable
internal class EditorExternalEmbedElementState {
  val assets = mutableStateMapOf<String, EditorEmbedAsset>()
  val unfurls = mutableStateMapOf<String, EditorEmbedUnfurl>()

  fun clear() {
    assets.clear()
    unfurls.clear()
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

internal data class EditorEmbedAsset(
  val id: String,
  val url: String,
  val title: String?,
  val description: String?,
  val thumbnailUrl: String?,
  val html: String?,
)

internal class EditorEmbedUnfurl

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
