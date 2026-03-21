package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.IntOffset
import co.typie.ext.toDp
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

data class PopoverListItem(
  val content: @Composable () -> Unit,
  val onSelected: () -> Unit,
)

@Composable
fun PopoverScope.PopoverList(
  items: List<PopoverListItem>,
) {
  val haptic = LocalHapticFeedback.current
  val density = LocalDensity.current
  val gestureScope = rememberCoroutineScope()
  val itemRadius = PopoverDefaults.ExpandedRadius - PopoverDefaults.PanePadding

  var activeIndex by remember { mutableStateOf<Int?>(null) }
  var isLocalTracking by remember { mutableStateOf(false) }
  val itemBounds = remember { mutableStateMapOf<Int, Rect>() }
  var listWindowOffset by remember { mutableStateOf(Offset.Zero) }

  // Indicator animation
  val indicatorX = remember { Animatable(0f) }
  val indicatorY = remember { Animatable(0f) }
  val indicatorW = remember { Animatable(0f) }
  val indicatorH = remember { Animatable(0f) }
  var indicatorVisible by remember { mutableStateOf(false) }

  val animSpec = tween<Float>(PopoverDefaults.IndicatorDuration, easing = EaseOutCubic)

  LaunchedEffect(activeIndex) {
    val index = activeIndex
    if (index == null) {
      indicatorVisible = false
      return@LaunchedEffect
    }
    val bounds = itemBounds[index] ?: return@LaunchedEffect
    val localLeft = bounds.left - listWindowOffset.x
    val localTop = bounds.top - listWindowOffset.y
    if (!indicatorVisible) {
      indicatorX.snapTo(localLeft)
      indicatorY.snapTo(localTop)
      indicatorW.snapTo(bounds.width)
      indicatorH.snapTo(bounds.height)
      indicatorVisible = true
    } else {
      launch { indicatorX.animateTo(localLeft, animSpec) }
      launch { indicatorY.animateTo(localTop, animSpec) }
      launch { indicatorW.animateTo(bounds.width, animSpec) }
      launch { indicatorH.animateTo(bounds.height, animSpec) }
    }
  }

  // Scope pointer tracking (drag from anchor)
  val currentPointerState = pointerState
  LaunchedEffect(currentPointerState) {
    if (isLocalTracking) {
      return@LaunchedEffect
    }

    val state = currentPointerState ?: run {
      activeIndex = null
      return@LaunchedEffect
    }

    if (!state.isSelectionArmed) {
      activeIndex = null
      return@LaunchedEffect
    }

    val index = hitTestItems(state.position, itemBounds)
    if (index != null && index != activeIndex) {
      haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
    }
    activeIndex = index

    if (state.isUp && state.isSelectionArmed) {
      val selectedIndex = activeIndex
      activeIndex = null
      if (selectedIndex != null) {
        items[selectedIndex].onSelected()
      }
    }
  }

  val colors = AppTheme.colors

  Box(
    modifier = Modifier
      .onGloballyPositioned { coordinates ->
        listWindowOffset = coordinates.positionInWindow()
      },
  ) {
    // Selection indicator
    if (indicatorVisible) {
      Box(
        modifier = Modifier
          .offset { IntOffset(indicatorX.value.toInt(), indicatorY.value.toInt()) }
          .width(indicatorW.value.toDp(density))
          .height(indicatorH.value.toDp(density))
          .background(colors.surfaceMuted, SquircleShape(itemRadius)),
      )
    }

    // Items
    Column(modifier = Modifier.fillMaxWidth()) {
      items.forEachIndexed { index, item ->
        Box(
          modifier = Modifier
            .fillMaxWidth()
            .onGloballyPositioned { coordinates ->
              val pos = coordinates.positionInWindow()
              val size = coordinates.size
              itemBounds[index] = Rect(
                pos.x, pos.y,
                pos.x + size.width, pos.y + size.height,
              )
            }
            .pointerInput(index) {
              awaitPointerEventScope {
                while (true) {
                  val event = awaitPointerEvent()
                  when (event.type) {
                    PointerEventType.Press -> {
                      val press = event.changes.firstOrNull() ?: continue
                      val pointerId = press.id
                      var currentWindowPos =
                        press.position + (itemBounds[index]?.topLeft ?: Offset.Zero)
                      var isSelectionArmed = false
                      isLocalTracking = true
                      activeIndex = null

                      val armJob = gestureScope.launch {
                        delay(PopoverDefaults.ArmDelayMs)
                        isSelectionArmed = true
                        val hitIndex = hitTestItems(currentWindowPos, itemBounds)
                        if (hitIndex != null && hitIndex != activeIndex) {
                          haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                        }
                        activeIndex = hitIndex
                      }

                      while (true) {
                        val moveEvent = awaitPointerEvent()
                        val change = moveEvent.changes.find { it.id == pointerId } ?: break

                        currentWindowPos =
                          change.position + (itemBounds[index]?.topLeft ?: Offset.Zero)
                        if (isSelectionArmed) {
                          val hitIndex = hitTestItems(currentWindowPos, itemBounds)
                          if (hitIndex != null && hitIndex != activeIndex) {
                            haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                          }
                          activeIndex = hitIndex
                        }

                        if (!change.pressed) {
                          armJob.cancel()
                          val selectedIndex = hitTestItems(currentWindowPos, itemBounds)
                          activeIndex = null
                          isLocalTracking = false
                          if (selectedIndex != null) {
                            items[selectedIndex].onSelected()
                          }
                          break
                        }
                      }

                      armJob.cancel()
                      activeIndex = null
                      isLocalTracking = false
                    }

                    else -> {}
                  }
                }
              }
            },
        ) {
          item.content()
        }
      }
    }
  }
}

internal fun hitTestItems(windowPosition: Offset, itemBounds: Map<Int, Rect>): Int? {
  for ((index, bounds) in itemBounds) {
    if (bounds.contains(windowPosition)) return index
  }
  return null
}
