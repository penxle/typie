package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Spinner
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage

@Composable
internal fun EditorImageExternalElement(
  data: ExternalElementData.Image,
  nodeId: String,
  size: Size?,
  zoom: Float,
) {
  val externalElementState = LocalEditorExternalElementState.current
  val imageState = externalElementState.images
  val upload = imageState.uploads[nodeId]
  val asset = data.id?.let(imageState.assets::get)
  val hasImage = asset != null || upload != null
  val resolution = data.id?.let(externalElementState.resolutions::get)
  val missingAsset = data.id != null && asset == null && upload == null
  val unavailableAsset =
    missingAsset &&
      (resolution == EditorAssetResolution.RetryableFailure ||
        resolution == EditorAssetResolution.Unavailable)
  val resolvingAsset = missingAsset && !unavailableAsset

  if (!hasImage) {
    ImagePlaceholder(resolvingAsset = resolvingAsset, unavailableAsset = unavailableAsset)
    return
  }

  if (size == null) return
  val imageShape = AppShapes.rounded(4.dp * zoom)

  Box(modifier = Modifier.fillMaxWidth().height(size.height.dp * zoom)) {
    Box(
      modifier =
        Modifier.align(Alignment.TopCenter)
          .width(size.width.dp * zoom)
          .height(size.height.dp * zoom)
          .clip(imageShape)
    ) {
      when {
        asset != null -> {
          Img(url = asset.url, modifier = Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        }
        upload != null -> {
          AsyncImage(
            model = upload.previewModel,
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop,
          )
        }
      }

      if (upload != null && asset == null) {
        Box(
          modifier = Modifier.fillMaxSize().background(Color.White.copy(alpha = 0.5f)),
          contentAlignment = Alignment.Center,
        ) {
          Spinner(
            color = AppTheme.colors.textHint,
            size = 24.dp * zoom,
            strokeWidth = 2.dp * zoom,
            sweepAngle = 270f,
          )
        }
      }
    }
  }
}

@Composable
private fun ImagePlaceholder(resolvingAsset: Boolean, unavailableAsset: Boolean) {
  EditorExternalElementPlaceholder(
    icon = Lucide.Image,
    text =
      when {
        unavailableAsset -> "이미지를 불러올 수 없어요"
        resolvingAsset -> "이미지를 불러오는 중..."
        else -> "이미지"
      },
    trailing = {
      if (resolvingAsset) {
        Spinner(
          color = AppTheme.colors.textHint,
          size = 16.dp,
          strokeWidth = 2.dp,
          sweepAngle = 270f,
        )
      }
    },
  )
}
