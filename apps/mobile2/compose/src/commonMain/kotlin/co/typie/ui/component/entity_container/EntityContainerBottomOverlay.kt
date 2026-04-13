package co.typie.ui.component.entity_container

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
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
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.EntityBottomOverlayDefaults
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

private val EntityContainerBottomOverlayHorizontalPadding = 16.dp
private val EntityContainerSelectionBarShape = RoundedCornerShape(999.dp)

@Composable
private fun EntityContainerBottomOverlayAnimatedSlot(
  visible: Boolean,
  slotHeight: Dp,
  label: String,
  content: (@Composable () -> Unit)?,
) {
  val targetHeight = if (visible && content != null) slotHeight else 0.dp
  val animatedHeight by
    animateDpAsState(
      targetValue = targetHeight,
      animationSpec =
        tween(
          if (targetHeight > 0.dp) {
            EntityBottomOverlayDefaults.EnterDurationMillis
          } else {
            EntityBottomOverlayDefaults.ExitDurationMillis
          }
        ),
      label = label,
    )

  Layout(
    content = {
      AnimatedVisibility(
        visible = visible && content != null,
        enter =
          fadeIn(animationSpec = tween(EntityBottomOverlayDefaults.EnterDurationMillis)) +
            slideInVertically(
              animationSpec = tween(EntityBottomOverlayDefaults.EnterDurationMillis),
              initialOffsetY = { it / 2 },
            ),
        exit =
          fadeOut(animationSpec = tween(EntityBottomOverlayDefaults.ExitDurationMillis)) +
            slideOutVertically(
              animationSpec = tween(EntityBottomOverlayDefaults.ExitDurationMillis),
              targetOffsetY = { it / 2 },
            ),
      ) {
        content?.invoke()
      }
    }
  ) { measurables, constraints ->
    val slotHeightPx = animatedHeight.roundToPx()
    val looseConstraints =
      constraints.copy(minWidth = 0, minHeight = 0, maxHeight = Constraints.Infinity)
    val placeable = measurables.singleOrNull()?.measure(looseConstraints)
    val layoutWidth = placeable?.width ?: 0

    layout(layoutWidth, slotHeightPx) {
      placeable?.placeRelative(x = 0, y = (slotHeightPx - placeable.height).coerceAtLeast(0))
    }
  }
}

@Composable
private fun EntityContainerBottomOverlayAnimatedGap(visible: Boolean) {
  val gapHeight by
    animateDpAsState(
      targetValue = if (visible) EntityBottomOverlayDefaults.Gap else 0.dp,
      animationSpec =
        tween(
          if (visible) {
            EntityBottomOverlayDefaults.EnterDurationMillis
          } else {
            EntityBottomOverlayDefaults.ExitDurationMillis
          }
        ),
      label = "entity-container-bottom-overlay-gap",
    )

  Spacer(Modifier.height(gapHeight))
}

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
      pasteBarHeight = EntityBottomOverlayDefaults.BarHeight,
      hasSelectionBar = showSelectionBar,
      selectionBarHeight = EntityBottomOverlayDefaults.BarHeight,
    )

  LaunchedEffect(metrics) { onMetricsChanged(metrics) }

  Box(modifier = modifier.fillMaxWidth(), contentAlignment = Alignment.BottomCenter) {
    Column(
      modifier =
        Modifier.padding(
          start = EntityContainerBottomOverlayHorizontalPadding,
          end = EntityContainerBottomOverlayHorizontalPadding,
          bottom = baseBottomInset + EntityBottomOverlayDefaults.BottomOffset,
        ),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      EntityContainerBottomOverlayAnimatedSlot(
        visible = showPasteBar,
        slotHeight = EntityBottomOverlayDefaults.BarHeight,
        label = "entity-container-bottom-overlay-paste-slot",
        content = pasteBar,
      )
      EntityContainerBottomOverlayAnimatedGap(visible = showPasteBar && showSelectionBar)
      EntityContainerBottomOverlayAnimatedSlot(
        visible = showSelectionBar,
        slotHeight = EntityBottomOverlayDefaults.BarHeight,
        label = "entity-container-bottom-overlay-selection-slot",
        content = selectionBar,
      )
    }
  }
}

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
          Modifier.height(EntityBottomOverlayDefaults.BarHeight)
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
          Modifier.size(EntityBottomOverlayDefaults.BarHeight)
            .clickable(onClick = onClearSelection)
            .pressScale(0.96f),
        contentAlignment = Alignment.Center,
      ) {
        Icon(icon = Lucide.X, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textPrimary)
      }
    }
  }
}
