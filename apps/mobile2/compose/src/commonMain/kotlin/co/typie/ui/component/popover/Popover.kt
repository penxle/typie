package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import androidx.compose.ui.window.Popup
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlin.time.TimeSource

data class AnchorPointerState(
  val position: Offset,
  val isSelectionArmed: Boolean,
  val isUp: Boolean,
)

@Stable
class PopoverScope internal constructor(
  private val onClose: () -> Unit,
) {
  var pointerState: AnchorPointerState? by mutableStateOf(null)
    internal set

  fun close() {
    onClose()
  }
}

@Composable
fun Popover(
  anchor: @Composable () -> Unit,
  pane: @Composable PopoverScope.() -> Unit,
  position: PopoverPosition = PopoverPosition.BottomRight,
  maxWidth: Dp? = null,
  collapsedCornerRadius: Dp = 0.dp,
) {
  val density = LocalDensity.current
  val screenPaddingPx = PopoverDefaults.ScreenPadding.toPx(density).toInt()

  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  var paneSize by remember { mutableStateOf<IntSize?>(null) }
  val animationProgress = remember { Animatable(0f) }

  val scope = remember {
    PopoverScope(onClose = { isExpanded = false })
  }

  // Drive close animation
  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      isOverlayVisible = true
      animationProgress.snapTo(0f)
    } else if (isOverlayVisible) {
      if (paneSize != null) {
        val from = if (animationProgress.value == 0f) 1f else animationProgress.value
        animationProgress.snapTo(from)
        animationProgress.animateTo(
          0f,
          tween(PopoverDefaults.ReverseDuration, easing = PopoverDefaults.PopoverEasing)
        )
      }
      isOverlayVisible = false
      paneSize = null
      scope.pointerState = null
    }
  }

  // Drive open animation once paneSize is measured
  LaunchedEffect(isExpanded, paneSize) {
    if (isExpanded && paneSize != null) {
      animationProgress.animateTo(
        1f,
        tween(PopoverDefaults.ForwardDuration, easing = PopoverDefaults.PopoverEasing)
      )
    }
  }

  PlatformBackHandler(enabled = isOverlayVisible) {
    isExpanded = false
  }

  // Anchor — Popup is placed inside so anchorBounds in PopupPositionProvider
  // correctly refers to this Box.
  Box(
    modifier = Modifier
      .onGloballyPositioned { coordinates ->
        val pos = coordinates.positionInWindow().round()
        anchorBounds = IntRect(pos, coordinates.size)
      }
      .pointerInput(Unit) {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent()
            if (event.type == PointerEventType.Press && !isOverlayVisible) {
              val press = event.changes.firstOrNull() ?: continue
              val anchorWindowOffset =
                Offset(anchorBounds.left.toFloat(), anchorBounds.top.toFloat())

              isExpanded = true
              scope.pointerState = AnchorPointerState(
                position = press.position + anchorWindowOffset,
                isSelectionArmed = false,
                isUp = false,
              )

              // Track pointer for drag-to-select
              val armStartMark = TimeSource.Monotonic.markNow()
              val origin = press.position + anchorWindowOffset
              var isArmed = false

              while (true) {
                val moveEvent = awaitPointerEvent()
                val change = moveEvent.changes.find { it.id == press.id } ?: break

                val currentPos = change.position + anchorWindowOffset
                val elapsed = armStartMark.elapsedNow().inWholeMilliseconds
                val distance = (currentPos - origin).getDistance()

                if (!isArmed && elapsed >= PopoverDefaults.ArmDelayMs && distance > PopoverDefaults.ArmDistance) {
                  isArmed = true
                }

                scope.pointerState = AnchorPointerState(
                  position = currentPos,
                  isSelectionArmed = isArmed,
                  isUp = !change.pressed,
                )

                if (!change.pressed) break
              }
            }
          }
        }
      },
  ) {
    // Anchor content — alpha applied to content only, not the container
    val anchorAlpha = if (isOverlayVisible && paneSize != null) {
      1f - PopoverDefaults.PopoverEasing.transform(animationProgress.value)
    } else {
      1f
    }
    Box(modifier = Modifier.alpha(anchorAlpha)) {
      anchor()
    }

    // Popup inside anchor Box — anchorBounds in calculatePosition will be this Box's bounds
    if (isOverlayVisible) {
      val colors = AppTheme.colors

      val positionProvider = remember(position, screenPaddingPx) {
        PopoverPositionProvider(position, screenPaddingPx)
      }

      Popup(
        popupPositionProvider = positionProvider,
        onDismissRequest = { isExpanded = false },
      ) {
        val measuredPaneSize = paneSize

        if (measuredPaneSize == null) {
          // Invisible measurement pass — IntrinsicSize.Max wraps to widest child
          Box(
            modifier = Modifier
              .width(IntrinsicSize.Max)
              .alpha(0f)
              .then(if (maxWidth != null) Modifier.widthIn(max = maxWidth) else Modifier)
              .onGloballyPositioned { coordinates ->
                paneSize = coordinates.size
              },
          ) {
            scope.pane()
          }
        } else {
          val progress =
            PopoverDefaults.PopoverEasing.transform(animationProgress.value).coerceIn(0f, 1f)
          val anchorSize = anchorBounds.size
          val animatedWidth =
            lerp(anchorSize.width.toFloat(), measuredPaneSize.width.toFloat(), progress)
          val animatedHeight =
            lerp(anchorSize.height.toFloat(), measuredPaneSize.height.toFloat(), progress)
          val cornerRadius = lerp(
            collapsedCornerRadius.toPx(density),
            PopoverDefaults.ExpandedRadius.toPx(density),
            progress
          )
          val shape = SquircleShape(cornerRadius.toDp(density))
          val shadowElevation = (12f * progress).toDp(density)

          val effective = effectivePosition(position, positionProvider.lastShowBelow)

          // Hide the morphing box when close animation completes but Popup hasn't
          // been removed yet (1-2 frame delay before recomposition sets isOverlayVisible=false)
          val dismissed = !isExpanded && progress <= 0f

          Box(
            modifier = Modifier
              .then(if (dismissed) Modifier.alpha(0f) else Modifier)
              .then(if (maxWidth != null) Modifier.widthIn(max = maxWidth) else Modifier)
              .size(
                width = animatedWidth.toDp(density),
                height = animatedHeight.toDp(density),
              )
              .shadow(shadowElevation, shape)
              .clip(shape)
              .background(colors.surfaceElevated, shape),
          ) {
            // Pane content (fades in)
            Box(modifier = Modifier.alpha(progress)) {
              scope.pane()
            }

            // Anchor ghost (fades out) — sized in exact pixels to avoid dp rounding blur
            val anchorOffset = anchorContentOffset(measuredPaneSize, anchorSize, effective)
            Layout(
              content = { anchor() },
              modifier = Modifier
                .offset { anchorOffset }
                .alpha(1f - progress),
            ) { measurables, _ ->
              val constraints = Constraints.fixed(anchorSize.width, anchorSize.height)
              val placeable = measurables.firstOrNull()?.measure(constraints)
              layout(anchorSize.width, anchorSize.height) {
                placeable?.place(0, 0)
              }
            }
          }
        }
      }
    }
  }
}

private fun lerp(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}

private fun anchorContentOffset(
  paneSize: IntSize,
  anchorSize: IntSize,
  position: PopoverPosition,
): IntOffset {
  val x = when (position) {
    PopoverPosition.BottomLeft, PopoverPosition.TopLeft -> 0
    PopoverPosition.BottomCenter, PopoverPosition.TopCenter -> (paneSize.width - anchorSize.width) / 2
    PopoverPosition.BottomRight, PopoverPosition.TopRight -> paneSize.width - anchorSize.width
  }
  val y = when (position) {
    PopoverPosition.BottomLeft, PopoverPosition.BottomCenter, PopoverPosition.BottomRight -> 0
    PopoverPosition.TopLeft, PopoverPosition.TopCenter, PopoverPosition.TopRight -> paneSize.height - anchorSize.height
  }
  return IntOffset(x, y)
}
