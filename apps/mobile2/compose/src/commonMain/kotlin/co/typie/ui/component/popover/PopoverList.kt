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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
  PopoverList(
    items = items,
    pointerState = pointerState,
    inputEnabled = acceptsInput,
  )
}

@Composable
fun PopoverList(
  items: List<PopoverListItem>,
  pointerState: AnchorPointerState?,
  inputEnabled: Boolean,
  armDelayMs: Long = PopoverDefaults.ArmDelayMs,
) {
  val haptic = LocalHapticFeedback.current
  val density = LocalDensity.current
  val gestureScope = rememberCoroutineScope()
  val edgeAutoScrollState = LocalPopoverPaneEdgeAutoScrollState.current
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

  fun updateActiveIndex(windowPosition: Offset) {
    val index = hitTestItems(windowPosition, itemBounds)
    if (index != null && index != activeIndex) {
      haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
    }
    activeIndex = index
  }

  fun updateEdgeAutoScroll(windowPosition: Offset) {
    edgeAutoScrollState?.update(
      pointerPosition = windowPosition,
      onAutoScroll = { updateActiveIndex(windowPosition) },
    )
  }

  DisposableEffect(edgeAutoScrollState) {
    onDispose {
      edgeAutoScrollState?.stop()
    }
  }

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
  LaunchedEffect(currentPointerState, inputEnabled) {
    if (!inputEnabled) {
      edgeAutoScrollState?.stop()
      activeIndex = null
      return@LaunchedEffect
    }

    if (isLocalTracking) {
      return@LaunchedEffect
    }

    val state = currentPointerState ?: run {
      edgeAutoScrollState?.stop()
      activeIndex = null
      return@LaunchedEffect
    }

    if (!state.isSelectionArmed) {
      edgeAutoScrollState?.stop()
      activeIndex = null
      return@LaunchedEffect
    }

    updateActiveIndex(state.position)
    if (!state.isUp) {
      updateEdgeAutoScroll(state.position)
    }

    if (state.isUp && state.isSelectionArmed) {
      edgeAutoScrollState?.stop()
      val selectedIndex = activeIndex
      activeIndex = null
      if (selectedIndex != null) {
        items[selectedIndex].onSelected()
      }
    }
  }

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
          .background(AppTheme.colors.surfaceTinted, SquircleShape(itemRadius)),
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
            .pointerInput(index, inputEnabled) {
              awaitPointerEventScope {
                while (true) {
                  val event = awaitPointerEvent()
                  if (!inputEnabled) {
                    event.changes.forEach { it.consume() }
                    continue
                  }

                  when (event.type) {
                    PointerEventType.Press -> {
                      val press = event.changes.firstOrNull() ?: continue
                      val pointerId = press.id
                      val originWindowPos =
                        press.position + (itemBounds[index]?.topLeft ?: Offset.Zero)
                      val touchSlop = viewConfiguration.touchSlop
                      var currentWindowPos =
                        originWindowPos
                      var isSelectionArmed = false
                      var isPanScroll = false
                      isLocalTracking = true
                      activeIndex = null

                      val armJob = gestureScope.launch {
                        delay(armDelayMs)
                        if (isPanScroll) {
                          return@launch
                        }
                        isSelectionArmed = true
                        updateActiveIndex(currentWindowPos)
                        updateEdgeAutoScroll(currentWindowPos)
                      }

                      while (true) {
                        val moveEvent = awaitPointerEvent()
                        val change = moveEvent.changes.find { it.id == pointerId } ?: break

                        currentWindowPos =
                          change.position + (itemBounds[index]?.topLeft ?: Offset.Zero)
                        if (!isSelectionArmed && !isPanScroll) {
                          val distance = (currentWindowPos - originWindowPos).getDistance()
                          if (distance > touchSlop) {
                            isPanScroll = true
                            armJob.cancel()
                            edgeAutoScrollState?.stop()
                            activeIndex = null
                          }
                        }
                        if (isSelectionArmed) {
                          change.consume()
                          updateActiveIndex(currentWindowPos)
                          updateEdgeAutoScroll(currentWindowPos)
                        }

                        if (!change.pressed) {
                          armJob.cancel()
                          edgeAutoScrollState?.stop()
                          val selectedIndex = when {
                            isPanScroll -> null
                            isSelectionArmed -> hitTestItems(currentWindowPos, itemBounds)
                            else -> hitTestItems(currentWindowPos, itemBounds)
                          }
                          activeIndex = null
                          isLocalTracking = false
                          if (selectedIndex != null) {
                            items[selectedIndex].onSelected()
                          }
                          break
                        }
                      }

                      armJob.cancel()
                      edgeAutoScrollState?.stop()
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
