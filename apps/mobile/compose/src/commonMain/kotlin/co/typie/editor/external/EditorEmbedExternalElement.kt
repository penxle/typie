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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import io.ktor.http.Url

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorEmbedExternalElement(data: ExternalElementData.Embed, nodeId: String) {
  val embedState = LocalEditorExternalElementState.current.embeds
  val asset = data.id?.let(embedState.assets::get)
  val unfurl = embedState.unfurls[nodeId]
  val resolvingAsset = data.id != null && asset == null && unfurl == null

  when {
    asset != null -> EmbedCard(asset)
    unfurl != null || resolvingAsset -> EmbedLoading()
    else -> EmbedPlaceholder()
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun EmbedPlaceholder() {
  EditorExternalElementPlaceholder(
    icon = Lucide.FileUp,
    text = "링크 임베드(Youtube, Google Drive, 일반 링크 등)",
  )
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun EmbedLoading() {
  EditorExternalElementPlaceholder(
    icon = Lucide.FileUp,
    text = "링크 임베드 중...",
    trailing = {
      Spinner(
        color = AppTheme.colors.textHint,
        size = scope.scaledDp(16f),
        strokeWidth = scope.scaledDp(2f),
        sweepAngle = 270f,
      )
    },
  )
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun EmbedCard(asset: EditorEmbedAsset) {
  val thumbnailUrl = asset.thumbnailUrl?.takeIf { it.isNotBlank() }
  val cardShape = AppShapes.rounded(scope.scaledDp(6f))
  val host = displayHost(asset.url)

  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
    Row(
      modifier =
        Modifier.widthIn(max = scope.scaledDp(600f))
          .fillMaxWidth()
          .heightIn(min = if (thumbnailUrl != null) scope.scaledDp(118f) else scope.scaledDp(0f))
          .clip(cardShape)
          .background(AppTheme.colors.surfaceDefault, cardShape)
          .border(1.dp, AppTheme.colors.borderHairline, cardShape),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Column(
        modifier =
          Modifier.weight(1f)
            .padding(horizontal = scope.scaledDp(16f), vertical = scope.scaledDp(15f))
      ) {
        Text(
          text = asset.title?.takeIf { it.isNotBlank() } ?: "(제목 없음)",
          style =
            AppTheme.typography.body.copy(
              fontSize = scope.scaledSp(14f),
              fontWeight = FontWeight.Medium,
            ),
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        val description = asset.description?.takeIf { it.isNotBlank() }
        if (description != null) {
          Spacer(Modifier.height(scope.scaledDp(3f)))
          Text(
            text = description,
            style =
              AppTheme.typography.caption.copy(
                fontSize = scope.scaledSp(12f),
                fontWeight = FontWeight.Medium,
              ),
            color = AppTheme.colors.textMuted,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
          )
        }
        Spacer(Modifier.height(scope.scaledDp(8f)))
        Text(
          text = host,
          style =
            AppTheme.typography.caption.copy(
              fontSize = scope.scaledSp(12f),
              fontWeight = FontWeight.Medium,
            ),
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }

      if (thumbnailUrl != null) {
        Box(
          modifier =
            Modifier.width(scope.scaledDp(118f))
              .height(scope.scaledDp(118f))
              .clip(RoundedCornerShape(topEnd = scope.scaledDp(5f), bottomEnd = scope.scaledDp(5f)))
        ) {
          Img(
            url = thumbnailUrl,
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
