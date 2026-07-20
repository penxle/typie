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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Spinner
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorImageExternalElement(
  data: ExternalElementData.Image,
  nodeId: String,
  boundsWidth: Float,
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
  val ratio = asset?.ratio ?: upload?.ratio

  if (!hasImage) {
    ImagePlaceholder(resolvingAsset = resolvingAsset, unavailableAsset = unavailableAsset)
    return
  }

  val imageRatio = ratio ?: return
  if (boundsWidth <= 0f || imageRatio <= 0.0) {
    return
  }

  val originalWidth = (asset?.width ?: upload?.width ?: 0).toFloat()
  val nodeProportion = data.proportion.coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
  val draftProportion = imageState.resizeDraftProportions[nodeId]
  val displayProportion = draftProportion ?: nodeProportion.toFloat()
  val displayWidth = imageResizeWidthForProportion(displayProportion, boundsWidth, originalWidth)
  val displayHeight = displayWidth / imageRatio.toFloat()
  val imageShape = AppShapes.rounded(scope.scaledDp(4f))

  Box(modifier = Modifier.fillMaxWidth().height(scope.scaledDp(displayHeight))) {
    Box(
      modifier =
        Modifier.align(Alignment.TopCenter)
          .width(scope.scaledDp(displayWidth))
          .height(scope.scaledDp(displayHeight))
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
            size = scope.scaledDp(24f),
            strokeWidth = scope.scaledDp(2f),
            sweepAngle = 270f,
          )
        }
      }
    }
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
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
          size = scope.scaledDp(16f),
          strokeWidth = scope.scaledDp(2f),
          sweepAngle = 270f,
        )
      }
    },
  )
}
