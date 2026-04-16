package co.typie.domain.entity

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private const val SHARE_THUMBNAIL_WIDTH_DP = 64
private const val SHARE_THUMBNAIL_HEIGHT_DP = 38

@Composable
internal fun ShareSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(16.dp),
    content = {
      Text(text = title, style = AppTheme.typography.caption, color = AppTheme.colors.textSecondary)
      content()
    },
  )
}

@Composable
internal fun ShareOptionRow(icon: IconData, label: String, trailing: @Composable () -> Unit) {
  Row(
    modifier = Modifier.fillMaxWidth().heightIn(min = 24.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = icon, modifier = Modifier.size(20.dp), tint = AppTheme.colors.textSecondary)

    Spacer(Modifier.size(8.dp))

    Text(
      text = label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )

    trailing()
  }
}

@Composable
internal fun ShareThumbnailControl(
  thumbnailUrl: String?,
  isMixed: Boolean,
  isUploading: Boolean,
  isRemoving: Boolean,
  onUploadClick: () -> Unit,
  onRemoveClick: () -> Unit,
) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    ShareThumbnailUploadButton(
      thumbnailUrl = thumbnailUrl,
      isMixed = isMixed,
      isUploading = isUploading,
      enabled = !isRemoving,
      onClick = onUploadClick,
    )

    if (!isMixed && thumbnailUrl != null) {
      ShareThumbnailRemoveButton(
        enabled = !isUploading && !isRemoving,
        isRemoving = isRemoving,
        onClick = onRemoveClick,
      )
    }
  }
}

@Composable
private fun ShareThumbnailUploadButton(
  thumbnailUrl: String?,
  isMixed: Boolean,
  isUploading: Boolean,
  enabled: Boolean,
  onClick: () -> Unit,
) {
  val shape = AppShapes.rounded(AppShapes.sm)

  InteractionScope {
    Box(
      modifier =
        Modifier.then(if (enabled) Modifier.clickable(onClick = onClick) else Modifier)
          .then(if (enabled) Modifier.pressScale(0.95f) else Modifier),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.size(width = SHARE_THUMBNAIL_WIDTH_DP.dp, height = SHARE_THUMBNAIL_HEIGHT_DP.dp)
            .clip(shape)
            .background(AppTheme.colors.surfaceSunken, shape)
            .border(
              width = 1.dp,
              color =
                if (thumbnailUrl == null) AppTheme.colors.borderStrong
                else AppTheme.colors.borderSubtle,
              shape = shape,
            ),
        contentAlignment = Alignment.Center,
      ) {
        when {
          isMixed -> {
            Text(
              text = if (isUploading) "..." else "다름",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textTertiary,
            )
          }

          thumbnailUrl != null -> {
            Img(url = thumbnailUrl, modifier = Modifier.fillMaxSize().clip(shape))
          }

          isUploading -> {
            ShareThumbnailSpinner()
          }

          else -> {
            Icon(
              icon = Lucide.Image,
              modifier = Modifier.size(14.dp),
              tint = AppTheme.colors.textTertiary,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun ShareThumbnailRemoveButton(enabled: Boolean, isRemoving: Boolean, onClick: () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.heightIn(min = SHARE_THUMBNAIL_HEIGHT_DP.dp)
          .clickable(enabled = enabled) { onClick() }
          .pressScale(0.95f),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = if (isRemoving) "삭제 중..." else "삭제",
        modifier = Modifier.padding(horizontal = 8.dp),
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W600),
        color = if (enabled && !isRemoving) AppTheme.colors.danger else AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
internal fun ShareThumbnailSpinner() {
  val transition = rememberInfiniteTransition()
  val spinnerColor = AppTheme.colors.textTertiary
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec = infiniteRepeatable(animation = tween(1000, easing = LinearEasing)),
    )

  Canvas(Modifier.size(14.dp)) {
    drawArc(
      color = spinnerColor,
      startAngle = rotation,
      sweepAngle = 220f,
      useCenter = false,
      style = Stroke(width = 1.5.dp.toPx(), cap = StrokeCap.Round),
    )
  }
}
