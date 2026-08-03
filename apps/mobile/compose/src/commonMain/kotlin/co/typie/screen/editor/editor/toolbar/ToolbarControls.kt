package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ext.LocalInteractionSource
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs

internal val ToolbarCapsuleShape = AppShapes.rounded(AppShapes.full)
internal val ToolbarButtonShape = AppShapes.circle
internal val ToolbarFixedActionShape = AppShapes.circle
internal val ToolbarIndicatorShape = AppShapes.circle
internal val ToolbarBottomPanelRadius = AppShapes.xl
internal val ToolbarBottomPanelShape = AppShapes.rounded(ToolbarBottomPanelRadius)

internal val ToolbarLabelTextStyle: TextStyle
  @Composable get() = AppTheme.typography.body.copy(lineHeight = 20.sp)

@Composable
internal fun EditorToolbarSurfaceBackground(shape: Shape, modifier: Modifier = Modifier) {
  Box(
    modifier =
      modifier
        .fillMaxSize()
        .background(
          color = AppTheme.colors.surfaceDefault.copy(alpha = ToolbarSurfaceOpacity),
          shape = shape,
        )
  )
}

@Composable
internal fun EditorToolbarButton(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  selected: Boolean = false,
  tint: Color? = null,
  enabled: Boolean = true,
) {
  EditorToolbarIconButton(
    icon = icon,
    contentDescription = contentDescription,
    onClick = onClick,
    shape = ToolbarButtonShape,
    selected = selected,
    tint = tint,
    enabled = enabled,
    modifier = modifier.size(ToolbarButtonSize),
  )
}

@Composable
internal fun EditorToolbarLabelButton(
  text: String,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  selected: Boolean = false,
  suffixIcon: IconData? = null,
  autoBringIntoView: Boolean = false,
  subtle: Boolean = false,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val bringIntoViewRequester = remember { BringIntoViewRequester() }
  val contentColor =
    when {
      selected -> AppTheme.colors.textDefault
      subtle -> AppTheme.colors.textHint
      else -> AppTheme.colors.textDefault
    }

  LaunchedEffect(autoBringIntoView, selected) {
    if (autoBringIntoView && selected) {
      bringIntoViewRequester.bringIntoView()
    }
  }

  Box(
    modifier =
      modifier
        .height(ToolbarButtonSize)
        .then(
          if (autoBringIntoView) Modifier.bringIntoViewRequester(bringIntoViewRequester)
          else Modifier
        )
        .focusProperties { canFocus = false }
        .semantics {
          this.contentDescription = contentDescription
          role = Role.Button
        }
        .clip(ToolbarButtonShape)
        .then(
          if (selected) {
            Modifier.background(AppTheme.colors.surfaceInset, ToolbarButtonShape)
          } else {
            Modifier
          }
        )
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick)
        .padding(horizontal = ToolbarLabelHorizontalPadding),
    contentAlignment = Alignment.Center,
  ) {
    Row(
      horizontalArrangement = Arrangement.spacedBy(4.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        text = text,
        style = ToolbarLabelTextStyle,
        color = contentColor,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
      if (suffixIcon != null) {
        Icon(
          icon = suffixIcon,
          contentDescription = null,
          modifier = Modifier.size(14.dp),
          tint = contentColor,
        )
      }
    }
  }
}

@Composable
internal fun EditorToolbarIconButton(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  shape: Shape,
  modifier: Modifier = Modifier,
  surface: Boolean = false,
  fixedActionSurface: Boolean = false,
  selected: Boolean = false,
  iconSize: Dp = ToolbarIconSize,
  tint: Color? = null,
  enabled: Boolean = true,
  inheritInteractionSource: Boolean = false,
  crossfadeIcon: Boolean = false,
) {
  val inheritedInteractionSource = LocalInteractionSource.current
  val localInteractionSource = remember { MutableInteractionSource() }
  val interactionSource =
    if (inheritInteractionSource && inheritedInteractionSource != null) {
      inheritedInteractionSource
    } else {
      localInteractionSource
    }
  val resolvedTint =
    tint
      ?: if (fixedActionSurface || selected || surface) {
        AppTheme.colors.textDefault
      } else {
        AppTheme.colors.textHint
      }
  val surfaceModifier =
    if (fixedActionSurface || selected || surface) {
      Modifier.background(AppTheme.colors.surfaceInset, shape)
        .then(
          if (fixedActionSurface) {
            Modifier.border(ToolbarBorderWidth, AppTheme.colors.borderDefault, shape)
          } else {
            Modifier
          }
        )
    } else {
      Modifier
    }

  Box(
    modifier =
      modifier
        .focusProperties { canFocus = false }
        .clip(shape)
        .then(surfaceModifier)
        .alpha(if (enabled) 1f else ToolbarDisabledOpacity)
        .clickable(
          enabled = enabled,
          interactionSource = interactionSource,
          indication = null,
          onClick = onClick,
        ),
    contentAlignment = Alignment.Center,
  ) {
    if (crossfadeIcon) {
      Crossfade(
        targetState = icon,
        animationSpec = tween(ToolbarFixedActionIconCrossfadeMillis),
        label = "EditorToolbarIconButtonIcon",
      ) { targetIcon ->
        Icon(
          icon = targetIcon,
          contentDescription = contentDescription,
          modifier = Modifier.size(iconSize),
          tint = resolvedTint,
        )
      }
    } else {
      Icon(
        icon = icon,
        contentDescription = contentDescription,
        modifier = Modifier.size(iconSize),
        tint = resolvedTint,
      )
    }
  }
}

@Composable
internal fun EditorToolbarPageIndicator(modifier: Modifier = Modifier) {
  Box(
    modifier = modifier.width(ToolbarPageIndicatorSlotWidth).fillMaxHeight(),
    contentAlignment = Alignment.Center,
  ) {
    Icon(
      icon = Lucide.Dot,
      contentDescription = null,
      modifier = Modifier.size(ToolbarIconSize),
      tint = AppTheme.colors.borderDefault,
    )
  }
}

@Composable
internal fun EditorToolbarDivider(modifier: Modifier = Modifier) {
  Box(
    modifier =
      modifier
        .width(ToolbarBorderWidth)
        .height(ToolbarDividerHeight)
        .background(AppTheme.colors.borderEmphasis)
  )
}

internal fun Modifier.emitPressInteractions(interactionSource: MutableInteractionSource): Modifier =
  pointerInput(interactionSource) {
    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      val press = PressInteraction.Press(down.position)
      interactionSource.tryEmit(press)

      val up = waitForUpOrCancellation()
      val release =
        if (up == null) {
          PressInteraction.Cancel(press)
        } else {
          PressInteraction.Release(press)
        }
      interactionSource.tryEmit(release)
    }
  }

@Composable
internal fun Modifier.toolbarVerticalSwipeGestures(
  onPointerSessionStart: () -> Unit,
  onSwipeUp: () -> Unit,
  onSwipeDown: () -> Unit,
  onSwipeDownProgress: (ToolbarDismissSwipeProgress) -> Unit,
  onSwipeDownCancelled: () -> Unit,
): Modifier {
  val latestOnPointerSessionStart = rememberUpdatedState(onPointerSessionStart)
  val latestOnSwipeUp = rememberUpdatedState(onSwipeUp)
  val latestOnSwipeDown = rememberUpdatedState(onSwipeDown)
  val latestOnSwipeDownProgress = rememberUpdatedState(onSwipeDownProgress)
  val latestOnSwipeDownCancelled = rememberUpdatedState(onSwipeDownCancelled)

  return pointerInput(Unit) {
    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
      latestOnPointerSessionStart.value()
      var lastPosition = down.position
      var totalDelta = Offset.Zero
      var direction = 0
      var dismissSwipeArmed = false

      fun updateDismissSwipe(distance: Float) {
        val nextArmed =
          if (dismissSwipeArmed) {
            distance > ToolbarDismissSwipeDisarmThreshold.toPx()
          } else {
            distance >= ToolbarDismissSwipeThreshold.toPx()
          }
        dismissSwipeArmed = nextArmed
        latestOnSwipeDownProgress.value(
          ToolbarDismissSwipeProgress(distancePx = distance, armed = nextArmed)
        )
      }

      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Initial)
        val change = event.changes.firstOrNull { it.id == down.id }
        if (change == null || change.isConsumed) {
          if (direction > 0) {
            latestOnSwipeDownCancelled.value()
          }
          break
        }

        totalDelta += change.position - lastPosition
        lastPosition = change.position

        if (!change.pressed) {
          if (direction > 0) {
            change.consume()
            if (dismissSwipeArmed) {
              latestOnSwipeDown.value()
            } else {
              latestOnSwipeDownCancelled.value()
            }
          }
          break
        }

        if (direction < 0) {
          change.consume()
          continue
        }

        if (direction > 0) {
          change.consume()
          updateDismissSwipe(totalDelta.y.coerceAtLeast(0f))
          continue
        }

        val horizontalDistance = abs(totalDelta.x)
        val verticalDistance = abs(totalDelta.y)
        if (maxOf(horizontalDistance, verticalDistance) <= viewConfiguration.touchSlop) {
          continue
        }
        if (verticalDistance < horizontalDistance * ToolbarDismissSwipeDirectionRatio) break

        change.consume()
        if (totalDelta.y < 0f) {
          direction = -1
          latestOnSwipeUp.value()
        } else {
          direction = 1
          updateDismissSwipe(totalDelta.y)
        }
      }
    }
  }
}

internal data class ToolbarDismissSwipeProgress(val distancePx: Float, val armed: Boolean)

internal fun Modifier.trackToolbarScrollGestureStart(
  onStart: () -> Unit,
  onEnd: () -> Unit,
): Modifier =
  pointerInput(Unit) {
    awaitEachGesture {
      awaitFirstDown(requireUnconsumed = false)
      onStart()

      try {
        do {
          val event = awaitPointerEvent(PointerEventPass.Final)
        } while (event.changes.any { it.pressed })
      } finally {
        onEnd()
      }
    }
  }

internal fun Modifier.preserveEditorFocusOnToolbarInteraction(): Modifier =
  pointerInput(Unit) {
    awaitPointerEventScope {
      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Main)
        event.changes.forEach { change ->
          if (change.pressed && !change.previousPressed) {
            change.consume()
          }
        }
      }
    }
  }
