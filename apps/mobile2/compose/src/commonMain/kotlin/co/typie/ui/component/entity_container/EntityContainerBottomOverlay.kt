package co.typie.ui.component.entity_container

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityPasteBarHeight
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

internal val EntityContainerSelectionBarHeight = EntityPasteBarHeight
private val EntityContainerBottomOverlayHorizontalPadding = 16.dp
private val EntityContainerSelectionBarShape = RoundedCornerShape(999.dp)
private const val EntityContainerBottomOverlayEnterDuration = 220
private const val EntityContainerBottomOverlayExitDuration = 180

@Composable
fun EntityContainerBottomOverlayStack(
  baseBottomInset: Dp,
  showSelectionBar: Boolean,
  showPasteBar: Boolean,
  modifier: Modifier = Modifier,
  selectionBar: (@Composable () -> Unit)? = null,
  pasteBar: (@Composable () -> Unit)? = null,
  onMetricsChanged: (EntityContainerBottomOverlayMetrics) -> Unit = {},
) {
  val metrics =
    calculateEntityContainerBottomOverlayMetrics(
      baseBottomInset = baseBottomInset,
      hasPasteBar = showPasteBar,
      pasteBarHeight = EntityContainerBottomOverlayBarHeight,
      hasSelectionBar = showSelectionBar,
      selectionBarHeight = EntityContainerSelectionBarHeight,
    )

  LaunchedEffect(metrics) { onMetricsChanged(metrics) }

  Box(modifier = modifier.fillMaxWidth(), contentAlignment = Alignment.BottomCenter) {
    Column(
      modifier =
        Modifier.padding(
          start = EntityContainerBottomOverlayHorizontalPadding,
          end = EntityContainerBottomOverlayHorizontalPadding,
          bottom = baseBottomInset + EntityContainerBottomOverlayBottomOffset,
        ),
      verticalArrangement = Arrangement.spacedBy(EntityContainerBottomOverlayGap),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      AnimatedVisibility(
        visible = showPasteBar && pasteBar != null,
        enter =
          fadeIn(animationSpec = tween(EntityContainerBottomOverlayEnterDuration)) +
            slideInVertically(
              animationSpec = tween(EntityContainerBottomOverlayEnterDuration),
              initialOffsetY = { it / 2 },
            ),
        exit =
          fadeOut(animationSpec = tween(EntityContainerBottomOverlayExitDuration)) +
            slideOutVertically(
              animationSpec = tween(EntityContainerBottomOverlayExitDuration),
              targetOffsetY = { it / 2 },
            ),
      ) {
        pasteBar?.invoke()
      }

      AnimatedVisibility(
        visible = showSelectionBar && selectionBar != null,
        enter =
          fadeIn(animationSpec = tween(EntityContainerBottomOverlayEnterDuration)) +
            slideInVertically(
              animationSpec = tween(EntityContainerBottomOverlayEnterDuration),
              initialOffsetY = { it / 2 },
            ),
        exit =
          fadeOut(animationSpec = tween(EntityContainerBottomOverlayExitDuration)) +
            slideOutVertically(
              animationSpec = tween(EntityContainerBottomOverlayExitDuration),
              targetOffsetY = { it / 2 },
            ),
      ) {
        selectionBar?.invoke()
      }
    }
  }
}

internal val EntityContainerBottomOverlayBarHeight = EntityPasteBarHeight

@Composable
fun EntityContainerSelectionBar(
  selectedCount: Int,
  modifier: Modifier = Modifier,
  onClearSelection: suspend () -> Unit,
  onMoreClick: suspend () -> Unit,
) {
  val colors = AppTheme.colors

  Row(
    modifier =
      modifier
        .dropShadow(EntityContainerSelectionBarShape) {
          color = colors.shadowAmbient
          radius = 3f
        }
        .dropShadow(EntityContainerSelectionBarShape) {
          color = colors.shadow
          offset = Offset(0f, 4f)
          radius = 16f
        }
        .background(AppTheme.colors.surfaceRaised, EntityContainerSelectionBarShape),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    InteractionScope {
      Box(
        modifier =
          Modifier.height(EntityContainerSelectionBarHeight)
            .clickable(onClick = onMoreClick)
            .pressScale(0.97f)
            .padding(start = 18.dp, end = 14.dp)
            .wrapContentWidth(),
        contentAlignment = Alignment.Center,
      ) {
        Row(
          horizontalArrangement = Arrangement.spacedBy(8.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Text(
            text = "${selectedCount}개 선택됨",
            style = AppTheme.typography.action,
            color = AppTheme.colors.textPrimary,
          )

          Icon(
            icon = Lucide.EllipsisVertical,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textPrimary,
          )
        }
      }
    }

    Box(
      modifier =
        Modifier.width(1.dp)
          .height(18.dp)
          .background(AppTheme.colors.borderStrong.copy(alpha = 0.4f))
    )

    InteractionScope {
      Box(
        modifier =
          Modifier.size(EntityContainerSelectionBarHeight)
            .clickable(onClick = onClearSelection)
            .pressScale(0.96f),
        contentAlignment = Alignment.Center,
      ) {
        Icon(icon = Lucide.X, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textPrimary)
      }
    }
  }
}
