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
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.safeDrawing
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.popover.rememberPressGestureSessionState
import co.typie.ui.component.popover.trackPressGestureSession
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

data class ActionMenuItem(
  val icon: IconData,
  val label: String,
  val tint: Color? = null,
  val onClick: () -> Unit = {},
)

private const val ACTION_SIZE = 56
private const val ACTION_GAP = 8
private const val ACTION_MENU_GAP = 10
private const val ACTION_SELECTION_ARM_DELAY_MS = 150L

internal const val ACTION_BUTTON_TOTAL_WIDTH = ACTION_SIZE + ACTION_GAP

@Composable
fun BottomBarActionButton(
  icon: IconData,
  menus: List<ActionMenuItem> = emptyList(),
  onClick: suspend () -> Unit = {},
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val actionInteractionSource = remember { MutableInteractionSource() }
  val actionScale = remember { Animatable(1f) }
  val isActionPressed by actionInteractionSource.collectIsPressedAsState()
  val safeBottomPadding = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()
  val hasMenu = menus.isNotEmpty()
  val pressGestureSessionState = rememberPressGestureSessionState()
  val menuSelectionState = rememberBottomBarMenuSelectionState()
  var isMenuOpen by remember(icon, menus) { mutableStateOf(false) }
  var isMenuPressed by remember { mutableStateOf(false) }
  var buttonWindowTopLeft by remember { mutableStateOf(Offset.Zero) }

  fun resetMenuGesture(resetBounds: Boolean = false) {
    pressGestureSessionState.clear()
    if (resetBounds) {
      menuSelectionState.reset()
    } else {
      menuSelectionState.clearPointer()
    }
    isMenuPressed = false
  }

  LaunchedEffect(icon, menus) {
    isMenuOpen = false
    resetMenuGesture(resetBounds = true)
  }

  val bottomBarEnabled = LocalBottomBarState.current?.enabled
  LaunchedEffect(bottomBarEnabled) {
    if (bottomBarEnabled != true) {
      isMenuOpen = false
      resetMenuGesture(resetBounds = true)
    }
  }

  LaunchedEffect(isMenuOpen, menus, pressGestureSessionState.session) {
    if (!isMenuOpen) {
      return@LaunchedEffect
    }

    val session = pressGestureSessionState.session
    val selectedIndex = menuSelectionState.syncSession(session)
    if (session == null) {
      return@LaunchedEffect
    }

    if (!session.isReleased) {
      return@LaunchedEffect
    }

    pressGestureSessionState.clear()
    selectedIndex?.let { index ->
      isMenuOpen = false
      menus.getOrNull(index)?.onClick?.invoke()
    }
  }

  LaunchedEffect(if (hasMenu) isMenuPressed else isActionPressed) {
    if (if (hasMenu) isMenuPressed else isActionPressed) {
      actionScale.animateTo(1.02f, tween(150, easing = EaseOutCubic))
    } else {
      actionScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  BoxWithConstraints(modifier = modifier.fillMaxSize()) {
    val shellWidth = (maxWidth - 48.dp).coerceAtMost(488.dp).coerceAtLeast(0.dp)
    val shellHorizontalInset = (maxWidth - shellWidth) / 2

    if (isMenuOpen && hasMenu) {
      Box(
        modifier =
          Modifier.fillMaxSize().pointerInput(Unit) {
            detectTapGestures {
              resetMenuGesture()
              isMenuOpen = false
            }
          }
      )
    }

    if (hasMenu) {
      AnimatedVisibility(
        visible = isMenuOpen,
        modifier =
          Modifier.align(Alignment.BottomEnd)
            .padding(
              end = shellHorizontalInset,
              bottom =
                safeBottomPadding +
                  BottomBarDefaults.BottomPadding +
                  ACTION_SIZE.dp +
                  ACTION_MENU_GAP.dp,
            ),
        enter =
          fadeIn(animationSpec = tween(280)) +
            slideInVertically(
              animationSpec = tween(280, easing = EaseOutCubic),
              initialOffsetY = { (it * 0.12f).toInt() },
            ),
        exit =
          fadeOut(animationSpec = tween(180)) +
            slideOutVertically(
              animationSpec = tween(180, easing = EaseOutCubic),
              targetOffsetY = { (it * 0.12f).toInt() },
            ),
      ) {
        Box(
          modifier =
            Modifier.dropShadow(AppShapes.squircle(AppShapes.xl)) {
                color = colors.shadow.copy(alpha = 0.08f)
                radius = 8f
              }
              .background(AppTheme.colors.surfaceRaised, AppShapes.squircle(AppShapes.xl))
              .border(1.dp, AppTheme.colors.borderDefault, AppShapes.squircle(AppShapes.xl))
        ) {
          Column(modifier = Modifier.width(IntrinsicSize.Max).padding(6.dp)) {
            menus.forEachIndexed { index, item ->
              Box(
                modifier =
                  Modifier.fillMaxWidth()
                    .onGloballyPositioned { coordinates ->
                      val position = coordinates.positionInWindow()
                      val size = coordinates.size
                      menuSelectionState.updateItemBounds(
                        index = index,
                        Rect(
                          left = position.x,
                          top = position.y,
                          right = position.x + size.width,
                          bottom = position.y + size.height,
                        ),
                      )
                    }
                    .background(
                      color =
                        if (menuSelectionState.activeIndex == index) {
                          AppTheme.colors.surfaceTinted
                        } else {
                          Color.Transparent
                        },
                      shape = AppShapes.squircle(AppShapes.md),
                    )
                    .clickable {
                      if (menuSelectionState.consumeSuppressedClick(index)) {
                        return@clickable
                      }
                      resetMenuGesture()
                      isMenuOpen = false
                      item.onClick()
                    }
              ) {
                ActionMenuItemRow(
                  item = item,
                  modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
                )
              }
            }
          }
        }
      }
    }

    CompositionLocalProvider(LocalInteractionSource provides actionInteractionSource) {
      Box(
        modifier =
          Modifier.align(Alignment.BottomEnd)
            .padding(
              end = shellHorizontalInset,
              bottom = safeBottomPadding + BottomBarDefaults.BottomPadding,
            )
            .size(ACTION_SIZE.dp)
            .onGloballyPositioned { coordinates ->
              buttonWindowTopLeft = coordinates.positionInWindow()
            }
            .graphicsLayer {
              scaleX = actionScale.value
              scaleY = actionScale.value
            }
            .dropShadow(AppShapes.circle) {
              color = colors.shadowAmbient
              radius = 3f
            }
            .dropShadow(AppShapes.circle) {
              color = colors.shadow
              offset = Offset(0f, 4f)
              radius = 16f
            }
            .background(AppTheme.colors.surfaceRaised, AppShapes.circle)
            .border(1.dp, AppTheme.colors.borderDefault.copy(alpha = 0.5f), AppShapes.circle)
            .then(
              if (hasMenu) {
                Modifier.pointerInput(icon, menus) {
                  awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    menuSelectionState.prepareGesture()

                    isMenuPressed = true
                    var released = false
                    try {
                      down.consume()
                      if (isMenuOpen) {
                        isMenuOpen = false
                        resetMenuGesture()
                        return@awaitEachGesture
                      }

                      isMenuOpen = true
                      released =
                        trackPressGestureSession(
                          pointerId = down.id,
                          initialPositionInWindow = buttonWindowTopLeft + down.position,
                          downUptimeMillis = down.uptimeMillis,
                          armDelayMillis = ACTION_SELECTION_ARM_DELAY_MS,
                          resolvePositionInWindow = { change, _ ->
                            buttonWindowTopLeft + change.position
                          },
                        ) { session, change ->
                          pressGestureSessionState.publish(session)
                          change?.consume()
                        }
                    } finally {
                      if (!released) {
                        pressGestureSessionState.clear()
                        menuSelectionState.clearPointer()
                      }
                      isMenuPressed = false
                    }
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
private fun ActionMenuItemRow(item: ActionMenuItem, modifier: Modifier = Modifier) {
  Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
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
