package co.typie.ui.component.bottombar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.safeDrawing
import co.typie.ext.toPx
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.popover.AnchorPointerState
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

data class ActionMenuItem(
  val icon: IconData,
  val label: String,
  val tint: Color? = null,
  val onClick: () -> Unit = {},
)

private const val ACTION_SIZE = 52
private const val ACTION_GAP = 8
private const val ACTION_MENU_GAP = 10
private const val ACTION_SELECTION_ARM_DELAY_MS = 150L
private const val ACTION_SAME_PRESS_SELECTION_DISTANCE_DP = 9

internal const val ACTION_BUTTON_TOTAL_WIDTH = ACTION_SIZE + ACTION_GAP

@Composable
fun BottomBarActionButton(
  icon: IconData,
  menus: List<ActionMenuItem> = emptyList(),
  onClick: () -> Unit = {},
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val density = LocalDensity.current
  val gestureScope = rememberCoroutineScope()
  val actionInteractionSource = remember { MutableInteractionSource() }
  val actionScale = remember { Animatable(1f) }
  val isActionPressed by actionInteractionSource.collectIsPressedAsState()
  val safeBottomPadding = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()
  val hasMenu = menus.isNotEmpty()
  val samePressSelectionDistance = ACTION_SAME_PRESS_SELECTION_DISTANCE_DP.dp.toPx(density)
  var isMenuOpen by remember(icon, menus) { mutableStateOf(false) }
  var isMenuPressed by remember { mutableStateOf(false) }
  var buttonWindowTopLeft by remember { mutableStateOf(Offset.Zero) }
  var menuPointerState by remember { mutableStateOf<AnchorPointerState?>(null) }
  var trackedPointerOrigin by remember { mutableStateOf<Offset?>(null) }
  var trackedPointerPosition by remember { mutableStateOf<Offset?>(null) }
  var isTrackedPointerArmed by remember { mutableStateOf(false) }
  var isTrackedPointerHoldComplete by remember { mutableStateOf(false) }
  var selectionArmJob by remember { mutableStateOf<Job?>(null) }

  fun updateTrackedPointerArmState(windowPosition: Offset) {
    if (isTrackedPointerArmed || !isTrackedPointerHoldComplete) {
      return
    }

    val origin = trackedPointerOrigin ?: return
    if ((windowPosition - origin).getDistance() <= samePressSelectionDistance) {
      return
    }

    isTrackedPointerArmed = true
    menuPointerState = AnchorPointerState(
      position = windowPosition,
      isSelectionArmed = true,
      isUp = false,
    )
  }

  fun resetTrackedPointer() {
    selectionArmJob?.cancel()
    selectionArmJob = null
    menuPointerState = null
    trackedPointerOrigin = null
    trackedPointerPosition = null
    isTrackedPointerArmed = false
    isTrackedPointerHoldComplete = false
    isMenuPressed = false
  }

  LaunchedEffect(icon, menus) {
    isMenuOpen = false
    resetTrackedPointer()
  }

  val bottomBarEnabled = LocalBottomBarState.current?.enabled
  LaunchedEffect(bottomBarEnabled) {
    if (bottomBarEnabled != true) {
      isMenuOpen = false
      resetTrackedPointer()
    }
  }

  LaunchedEffect(if (hasMenu) isMenuPressed else isActionPressed) {
    if (if (hasMenu) isMenuPressed else isActionPressed) {
      actionScale.animateTo(1.05f, tween(150, easing = EaseOutCubic))
    } else {
      actionScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  BoxWithConstraints(
    modifier = modifier
      .fillMaxSize(),
  ) {
    val shellWidth = (maxWidth - 48.dp).coerceAtMost(488.dp).coerceAtLeast(0.dp)
    val shellHorizontalInset = (maxWidth - shellWidth) / 2

    if (isMenuOpen && hasMenu) {
      Box(
        modifier = Modifier
          .fillMaxSize()
          .pointerInput(Unit) {
            detectTapGestures {
              resetTrackedPointer()
              isMenuOpen = false
            }
          },
      )
    }

    if (hasMenu) {
      AnimatedVisibility(
        visible = isMenuOpen,
        modifier = Modifier
          .align(Alignment.BottomEnd)
          .padding(
            end = shellHorizontalInset,
            bottom = safeBottomPadding + BottomBarDefaults.BottomPadding + ACTION_SIZE.dp + ACTION_MENU_GAP.dp,
          ),
        enter = fadeIn(animationSpec = tween(280)) + slideInVertically(
          animationSpec = tween(280, easing = EaseOutCubic),
          initialOffsetY = { (it * 0.12f).toInt() },
        ),
        exit = fadeOut(animationSpec = tween(180)) + slideOutVertically(
          animationSpec = tween(180, easing = EaseOutCubic),
          targetOffsetY = { (it * 0.12f).toInt() },
        ),
      ) {
        Box(
          modifier = Modifier
            .dropShadow(SquircleShape(22.dp)) {
              color = colors.shadow.copy(alpha = 0.08f)
              radius = 8f
            }
            .background(AppTheme.colors.surfaceRaised, SquircleShape(22.dp))
            .border(1.dp, AppTheme.colors.borderDefault, SquircleShape(22.dp)),
        ) {
          Column(
            modifier = Modifier
              .width(IntrinsicSize.Max)
              .padding(6.dp),
          ) {
            PopoverList(
              items = menus.map { item ->
                PopoverListItem(
                  content = {
                    ActionMenuItemRow(
                      item = item,
                      modifier = Modifier
                        .height(42.dp)
                        .padding(horizontal = 16.dp),
                    )
                  },
                  onSelected = {
                    resetTrackedPointer()
                    isMenuOpen = false
                    item.onClick()
                  },
                )
              },
              pointerState = menuPointerState,
              inputEnabled = isMenuOpen,
              armDelayMs = ACTION_SELECTION_ARM_DELAY_MS,
            )
          }
        }
      }
    }

    CompositionLocalProvider(LocalInteractionSource provides actionInteractionSource) {
      Box(
        modifier = Modifier
          .align(Alignment.BottomEnd)
          .padding(end = shellHorizontalInset, bottom = safeBottomPadding + BottomBarDefaults.BottomPadding)
          .size(ACTION_SIZE.dp)
          .onGloballyPositioned { coordinates ->
            buttonWindowTopLeft = coordinates.positionInWindow()
          }
          .graphicsLayer {
            scaleX = actionScale.value
            scaleY = actionScale.value
          }
          .dropShadow(CircleShape) {
            color = colors.shadowAmbient
            radius = 3f
          }
          .dropShadow(CircleShape) {
            color = colors.shadow
            offset = Offset(0f, 4f)
            radius = 16f
          }
          .background(AppTheme.colors.surfaceRaised, CircleShape)
          .border(1.dp, AppTheme.colors.borderDefault.copy(alpha = 0.5f), CircleShape)
          .then(
            if (hasMenu) {
              Modifier.pointerInput(icon, menus) {
                awaitEachGesture {
                  val down = awaitFirstDown(requireUnconsumed = false)

                  isMenuPressed = true
                  down.consume()
                  if (isMenuOpen) {
                    isMenuOpen = false
                    resetTrackedPointer()
                  } else {
                    trackedPointerOrigin = buttonWindowTopLeft + down.position
                    trackedPointerPosition = trackedPointerOrigin
                    isTrackedPointerArmed = false
                    isTrackedPointerHoldComplete = false
                    menuPointerState = AnchorPointerState(
                      position = trackedPointerOrigin!!,
                      isSelectionArmed = false,
                      isUp = false,
                    )
                    isMenuOpen = true

                    selectionArmJob?.cancel()
                    selectionArmJob = gestureScope.launch {
                      delay(ACTION_SELECTION_ARM_DELAY_MS)
                      isTrackedPointerHoldComplete = true
                      trackedPointerPosition?.let { updateTrackedPointerArmState(it) }
                    }
                  }

                  while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.find { it.id == down.id } ?: break
                    val windowPosition = buttonWindowTopLeft + change.position
                    trackedPointerPosition = windowPosition
                    updateTrackedPointerArmState(windowPosition)
                    menuPointerState = AnchorPointerState(
                      position = windowPosition,
                      isSelectionArmed = isTrackedPointerArmed,
                      isUp = !change.pressed,
                    )
                    if (isTrackedPointerArmed) change.consume()
                    if (!change.pressed) {
                      selectionArmJob?.cancel()
                      selectionArmJob = null
                      trackedPointerOrigin = null
                      trackedPointerPosition = null
                      isTrackedPointerArmed = false
                      isTrackedPointerHoldComplete = false
                      isMenuPressed = false
                      break
                    }
                  }

                  isMenuPressed = false
                }
              }
            } else {
              Modifier.clickable { onClick() }
            }
          ),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = if (hasMenu && isMenuOpen) Lucide.X else icon,
          tint = AppTheme.colors.textSecondary,
        )
      }
    }
  }
}

@Composable
private fun ActionMenuItemRow(
  item: ActionMenuItem,
  modifier: Modifier = Modifier,
) {
  Row(
    modifier = modifier,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = item.icon,
      modifier = Modifier.size(18.dp),
      tint = item.tint ?: AppTheme.colors.textPrimary,
    )

    Spacer(Modifier.width(12.dp))

    Text(
      text = item.label,
      style = AppTheme.typography.action,
      color = item.tint ?: AppTheme.colors.textPrimary,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}
