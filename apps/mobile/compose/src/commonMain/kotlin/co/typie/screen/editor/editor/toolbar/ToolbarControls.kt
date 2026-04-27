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
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import co.typie.ext.LocalInteractionSource
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

internal val ToolbarCapsuleShape = AppShapes.rounded(AppShapes.full)
internal val ToolbarButtonShape = AppShapes.circle
internal val ToolbarFixedActionShape = AppShapes.circle
internal val ToolbarIndicatorShape = AppShapes.circle
internal val ToolbarBottomPanelShape = AppShapes.rounded(AppShapes.xl)

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
) {
  EditorToolbarIconButton(
    icon = icon,
    contentDescription = contentDescription,
    onClick = onClick,
    shape = ToolbarButtonShape,
    selected = selected,
    tint = tint,
    modifier = modifier.size(ToolbarButtonSize),
  )
}

@Composable
internal fun EditorToolbarLabelButton(
  text: String,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val interactionSource = remember { MutableInteractionSource() }

  Box(
    modifier =
      modifier
        .height(ToolbarButtonSize)
        .widthIn(min = ToolbarLabelMinWidth)
        .focusProperties { canFocus = false }
        .clip(ToolbarButtonShape)
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick)
        .padding(horizontal = ToolbarLabelHorizontalPadding),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = text,
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textDefault,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
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
  val surfaceModifier =
    when {
      fixedActionSurface ->
        Modifier.background(AppTheme.colors.surfaceInset, shape)
          .border(ToolbarBorderWidth, AppTheme.colors.borderDefault, shape)
      selected -> Modifier.background(AppTheme.colors.surfaceInset, shape)
      surface -> Modifier.background(AppTheme.colors.surfaceInset, shape)
      else -> Modifier
    }

  Box(
    modifier =
      modifier
        .focusProperties { canFocus = false }
        .clip(shape)
        .then(surfaceModifier)
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick),
    contentAlignment = Alignment.Center,
  ) {
    val resolvedTint = tint ?: AppTheme.colors.textDefault
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
