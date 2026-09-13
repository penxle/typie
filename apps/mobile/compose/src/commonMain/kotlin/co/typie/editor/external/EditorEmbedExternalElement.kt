package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import coil3.compose.LocalPlatformContext
import coil3.request.ImageRequest
import io.ktor.http.Url
import kotlin.math.roundToInt

@Composable
internal fun EditorEmbedExternalElement(
  data: ExternalElementData.Embed,
  nodeId: String,
  zoom: Float,
) {
  val externalElementState = LocalEditorExternalElementState.current
  val embedState = externalElementState.embeds
  val asset = data.id?.let(embedState.assets::get)
  val unfurl = embedState.unfurls[nodeId]
  val resolution = data.id?.let(externalElementState.resolutions::get)
  val missingAsset = data.id != null && asset == null && unfurl == null
  val unavailableAsset =
    missingAsset &&
      (resolution == EditorAssetResolution.RetryableFailure ||
        resolution == EditorAssetResolution.Unavailable)
  val resolvingAsset = missingAsset && !unavailableAsset

  when {
    asset != null -> EmbedCard(asset, zoom)
    unfurl != null || resolvingAsset -> EmbedLoading()
    unavailableAsset -> EmbedUnavailable()
    else -> EmbedPlaceholder()
  }
}

@Composable
private fun EmbedPlaceholder() {
  EditorExternalElementPlaceholder(
    icon = Lucide.FileUp,
    text = "링크 임베드(Youtube, Google Drive, 일반 링크 등)",
  )
}

@Composable
private fun EmbedLoading() {
  EditorExternalElementPlaceholder(
    icon = Lucide.FileUp,
    text = "링크 임베드 중...",
    trailing = {
      Spinner(color = AppTheme.colors.textHint, size = 16.dp, strokeWidth = 2.dp, sweepAngle = 270f)
    },
  )
}

@Composable
private fun EmbedUnavailable() {
  EditorExternalElementPlaceholder(icon = Lucide.FileUp, text = "링크를 불러올 수 없어요")
}

@Composable
private fun EmbedCard(asset: EditorEmbedAsset, zoom: Float) {
  val thumbnailUrl = asset.thumbnailUrl?.takeIf { it.isNotBlank() }
  val cardShape = AppShapes.rounded(6.dp)
  val host = displayHost(asset.url)

  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
    Row(
      modifier =
        Modifier.widthIn(max = 600.dp)
          .fillMaxWidth()
          .heightIn(min = if (thumbnailUrl != null) 118.dp else 0.dp)
          .clip(cardShape)
          .background(AppTheme.colors.surfaceDefault, cardShape)
          .border(1.dp, AppTheme.colors.borderHairline, cardShape),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Column(modifier = Modifier.weight(1f).padding(horizontal = 16.dp, vertical = 15.dp)) {
        Text(
          text = asset.title?.takeIf { it.isNotBlank() } ?: "(제목 없음)",
          style = AppTheme.typography.body.copy(fontSize = 14.sp, fontWeight = FontWeight.Medium),
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        val description = asset.description?.takeIf { it.isNotBlank() }
        if (description != null) {
          Spacer(Modifier.height(3.dp))
          Text(
            text = description,
            style =
              AppTheme.typography.caption.copy(fontSize = 12.sp, fontWeight = FontWeight.Medium),
            color = AppTheme.colors.textMuted,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
          )
        }
        Spacer(Modifier.height(8.dp))
        Text(
          text = host,
          style =
            AppTheme.typography.caption.copy(fontSize = 12.sp, fontWeight = FontWeight.Medium),
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }

      if (thumbnailUrl != null) {
        val context = LocalPlatformContext.current
        val thumbnailPixels =
          with(LocalDensity.current) { (118.dp.toPx() * zoom).roundToInt().coerceAtLeast(1) }
        val thumbnailRequest =
          remember(context, thumbnailUrl, thumbnailPixels) {
            ImageRequest.Builder(context).data(thumbnailUrl).size(thumbnailPixels).build()
          }
        Box(
          modifier =
            Modifier.width(118.dp)
              .height(118.dp)
              .clip(RoundedCornerShape(topEnd = 5.dp, bottomEnd = 5.dp))
        ) {
          AsyncImage(
            model = thumbnailRequest,
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop,
          )
        }
      }
    }
  }
}

private fun displayHost(url: String): String =
  runCatching { Url(url).host.takeIf { it.isNotBlank() } }.getOrNull() ?: url
