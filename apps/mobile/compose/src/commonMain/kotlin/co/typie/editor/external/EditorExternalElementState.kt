package co.typie.editor.external

import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.ExternalElementData

@Stable
internal class EditorExternalElementState {
  val images = EditorExternalImageElementState()
  val files = EditorExternalFileElementState()
  val embeds = EditorExternalEmbedElementState()
  val resolutions = mutableStateMapOf<String, EditorAssetResolution>()

  fun put(asset: EditorExternalAsset) {
    when (asset) {
      is EditorImageAsset -> images.assets[asset.id] = asset
      is EditorFileAsset -> files.assets[asset.id] = asset
      is EditorEmbedAsset -> embeds.assets[asset.id] = asset
    }
  }

  fun clear() {
    images.clear()
    files.clear()
    embeds.clear()
    resolutions.clear()
  }

  fun containsAsset(id: String): Boolean =
    images.assets.containsKey(id) || files.assets.containsKey(id) || embeds.assets.containsKey(id)
}

internal sealed interface EditorAssetResolution {
  data object InFlight : EditorAssetResolution

  data object RetryableFailure : EditorAssetResolution

  data class AwaitingMaterialization(val attempt: Int) : EditorAssetResolution

  data object Unavailable : EditorAssetResolution
}

@Stable
internal class EditorExternalImageElementState {
  val assets = mutableStateMapOf<String, EditorImageAsset>()
  val uploads = mutableStateMapOf<String, EditorImageUpload>()
  val resizeDrafts = mutableStateMapOf<String, EditorImageResizeDraft>()

  fun displaySize(nodeId: String, data: ExternalElementData.Image, boundsWidth: Float): Size? {
    val asset = data.id?.let(assets::get)
    val upload = uploads[nodeId]
    val originalWidth = (asset?.width ?: upload?.width ?: return null).toFloat()
    val ratio = (asset?.ratio ?: upload?.ratio ?: return null).toFloat()
    if (boundsWidth <= 0f || originalWidth <= 0f || !ratio.isFinite() || ratio <= 0f) return null
    val draft = resizeDrafts[nodeId]
    val maxSize =
      draft?.maxSize ?: imageResizeMaxSize(boundsWidth, originalWidth, ratio, data.maxHeight)
    return imageResizeSize(draft?.proportion ?: data.proportion.toFloat(), maxSize)
  }

  fun clearResizeState(nodeId: String) {
    resizeDrafts.remove(nodeId)
  }

  fun clear() {
    assets.clear()
    uploads.clear()
    resizeDrafts.clear()
  }
}

internal data class EditorImageResizeDraft(val proportion: Float, val maxSize: Size)

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

internal sealed interface EditorExternalAsset {
  val id: String
}

internal data class EditorFileAsset(
  override val id: String,
  val name: String,
  val url: String,
  val size: Long?,
) : EditorExternalAsset

internal data class EditorImageAsset(
  override val id: String,
  val url: String,
  val originalUrl: String,
  val width: Int,
  val height: Int,
  val ratio: Double,
  val placeholder: String?,
) : EditorExternalAsset

internal data class EditorEmbedAsset(
  override val id: String,
  val url: String,
  val title: String?,
  val description: String?,
  val thumbnailUrl: String?,
  val html: String?,
) : EditorExternalAsset

internal class EditorEmbedUnfurl

internal class EditorImageUpload(
  val previewModel: Any,
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
