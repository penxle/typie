package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorFileExternalElement(data: ExternalElementData.File, nodeId: String) {
  val externalElementState = LocalEditorExternalElementState.current
  val fileState = externalElementState.files
  val upload = fileState.uploads[nodeId]
  val asset = data.id?.let(fileState.assets::get)
  val hasFile = asset != null || upload != null
  val resolution = data.id?.let(externalElementState.resolutions::get)
  val missingAsset = data.id != null && asset == null && upload == null
  val unavailableAsset =
    missingAsset &&
      (resolution == EditorAssetResolution.RetryableFailure ||
        resolution == EditorAssetResolution.Unavailable)
  val resolvingAsset = missingAsset && !unavailableAsset
  val displayName = asset?.name ?: upload?.name ?: "파일"
  val displaySize = formatFileSize(asset?.size ?: upload?.size)

  if (!hasFile) {
    FilePlaceholder(resolvingAsset = resolvingAsset, unavailableAsset = unavailableAsset)
    return
  }

  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
    val cardShape = AppShapes.rounded(scope.scaledDp(8f))
    Row(
      modifier =
        Modifier.widthIn(max = scope.scaledDp(400f))
          .fillMaxWidth()
          .height(scope.scaledDp(64f))
          .clip(cardShape)
          .background(AppTheme.colors.surfaceInset, cardShape)
          .border(1.dp, AppTheme.colors.borderHairline, cardShape)
          .padding(horizontal = scope.scaledDp(16f)),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(
        icon = Lucide.File,
        contentDescription = null,
        modifier = Modifier.size(scope.scaledDp(20f)),
        tint = AppTheme.colors.textMuted,
      )
      Column(modifier = Modifier.padding(start = scope.scaledDp(12f)).weight(1f)) {
        Text(
          text = displayName,
          color = AppTheme.colors.textDefault,
          style =
            AppTheme.typography.body.copy(
              fontSize = scope.scaledSp(14f),
              fontWeight = FontWeight.Medium,
            ),
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        if (displaySize != null) {
          Text(
            text = displaySize,
            color = AppTheme.colors.textMuted,
            style = AppTheme.typography.caption.copy(fontSize = scope.scaledSp(12f)),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
      if (upload != null && asset == null) {
        Spinner(
          color = AppTheme.colors.textHint,
          modifier = Modifier.padding(start = scope.scaledDp(12f)),
          size = scope.scaledDp(20f),
          strokeWidth = scope.scaledDp(2f),
          sweepAngle = 270f,
        )
      }
    }
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun FilePlaceholder(resolvingAsset: Boolean, unavailableAsset: Boolean) {
  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
    Box(modifier = Modifier.widthIn(max = scope.scaledDp(400f)).fillMaxWidth()) {
      EditorExternalElementPlaceholder(
        icon = Lucide.File,
        text =
          when {
            unavailableAsset -> "파일을 불러올 수 없어요"
            resolvingAsset -> "파일을 불러오는 중..."
            else -> "파일"
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
  }
}

private fun formatFileSize(size: Long?): String? {
  if (size == null || size <= 0L) {
    return null
  }

  val units = listOf("B", "KB", "MB", "GB")
  var unitIndex = 0
  var fileSize = size.toDouble()

  while (fileSize >= 1024 && unitIndex < units.lastIndex) {
    fileSize /= 1024
    unitIndex++
  }

  if (unitIndex == 0) {
    return "${fileSize.toInt()} ${units[unitIndex]}"
  }
  return "${fileSize.asOneDecimal()} ${units[unitIndex]}"
}

private fun Double.asOneDecimal(): String {
  val rounded = (this * 10).roundToInt() / 10.0
  val whole = rounded.toInt()
  val fraction = ((rounded - whole) * 10).roundToInt()
  return "$whole.$fraction"
}
