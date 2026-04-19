package co.typie.shell

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch

@Composable
internal fun MainBottomBarPillEffects(
  state: MainBottomBarPillState,
  currentTab: Tab,
  isPillPressed: Boolean,
) {
  LaunchedEffect(isPillPressed) { state.animatePillScale(isPillPressed) }
  LaunchedEffect(currentTab) { state.startTransition(currentTab) }
}

internal fun Modifier.mainBottomBarPillGestures(
  state: MainBottomBarPillState,
  tabCenters: Map<Tab, Float>,
  tabWidths: Map<Tab, Float>,
  indicatorInsetPx: Float,
  totalWidth: Float,
  tabState: TabState,
): Modifier =
  pointerInput(
    tabCenters,
    tabWidths,
    indicatorInsetPx,
    totalWidth,
    tabState.currentTab,
    tabState.onSelectTab,
  ) {
    if (totalWidth <= 0f) return@pointerInput

    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      val pressInteraction = PressInteraction.Press(down.position)
      val trackHeightPx = size.height.toFloat()
      val downX = down.position.x.coerceIn(0f, totalWidth)
      var trackedX = downX
      var lastClampedPosition = Offset(x = downX, y = down.position.y.coerceIn(0f, trackHeightPx))
      var totalDelta = Offset.Zero
      var dragging = false
      var released = false

      val currentBaseCenter = tabCenters[tabState.currentTab] ?: 0f
      state.beginGesture(pressInteraction, currentBaseCenter, downX)

      try {
        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.firstOrNull { it.id == down.id } ?: break

          if (change.changedToUp()) {
            val releaseX = change.position.x.coerceIn(0f, totalWidth)
            val pointerAtRelease = if (dragging) releaseX else state.dragPointerX
            val (snapshotLeft, snapshotRight) =
              snapshotBoundsAt(pointerAtRelease, tabCenters, tabWidths, indicatorInsetPx)
            released = true
            state.releaseGesture(
              pressInteraction = pressInteraction,
              snapshotLeft = snapshotLeft,
              snapshotRight = snapshotRight,
              targetTab = nearestTab(tabCenters, totalWidth, releaseX),
              onSelectTab = tabState.onSelectTab,
            )
            break
          }

          if (change.isConsumed) break

          val clampedPosition =
            Offset(
              x = change.position.x.coerceIn(0f, totalWidth),
              y = change.position.y.coerceIn(0f, trackHeightPx),
            )
          val delta = clampedPosition - lastClampedPosition
          totalDelta += delta
          lastClampedPosition = clampedPosition
          val pointerX = clampedPosition.x

          // Treat as movement only when the pointer crossed a 1px threshold. Touch sensors
          // emit synthetic sub-pixel jitter for stationary fingers; without this filter
          // the very first jitter event would kill the down-follow animation (via
          // dragPointerJob.cancel) and snap dragPointerX to the touch position — defeating
          // the smooth tap catch-up. 1px is small enough that any real finger movement
          // (typically several px/frame) reliably triggers follow.
          if (delta.getDistance() > MainBottomBarPillFollowMovementThresholdPx) {
            trackedX = state.followDrag(trackedX, pointerX, totalWidth)
          }

          if (!dragging) {
            if (abs(totalDelta.x) > viewConfiguration.touchSlop) {
              dragging = true
              change.consume()
            } else if (abs(totalDelta.y) > viewConfiguration.touchSlop) {
              break
            }
          } else {
            change.consume()
          }
        }
      } finally {
        if (!released) {
          val (snapshotLeft, snapshotRight) =
            snapshotBoundsAt(state.dragPointerX, tabCenters, tabWidths, indicatorInsetPx)
          state.cancelGesture(
            pressInteraction = pressInteraction,
            snapshotLeft = snapshotLeft,
            snapshotRight = snapshotRight,
            currentActiveTab = tabState.currentTab,
          )
        }
      }
    }
  }

@Composable
internal fun rememberMainBottomBarPillState(initialActiveTab: Tab): MainBottomBarPillState {
  val scope = rememberCoroutineScope()
  return remember(scope) { MainBottomBarPillState(scope, initialActiveTab) }
}

internal class MainBottomBarPillState(private val scope: CoroutineScope, initialActiveTab: Tab) {
  val interactionSource = MutableInteractionSource()
  val pillScale = Animatable(1f)

  // Per-tab "active progress" 0..1. Drives tab box widths.
  val tabProgress = Tab.entries.associateWith { Animatable(if (it == initialActiveTab) 1f else 0f) }

  // 0 = at snapshotPrev (release point); 1 = at the new active tab's natural bounds.
  val transitionProgress = Animatable(1f)

  // Snapshot of indicator bounds at the moment of the latest activeTab change. The
  // indicator morphs from these bounds → new active tab's natural bounds as
  // transitionProgress animates 0→1, in lockstep with the tab box width animations.
  private var snapshotPrevLeft by mutableFloatStateOf(0f)
  private var snapshotPrevRight by mutableFloatStateOf(0f)

  // Sync flag set in release/cancel, cleared in startTransition once transitionProgress
  // has been snapped to 0. While set, indicatorShape bypasses the lerp formula and
  // renders the snapshot directly — protects against a 1-frame jump in the recomposition
  // that runs before the launched startTransition coroutine has executed snapTo.
  private var transitionPending by mutableStateOf(false)
  private var lastTransitionedToTab: Tab = initialActiveTab

  // Drag follower. On press, animated from currentBaseCenter → downX (smooth catch-up
  // for taps). On any movement, snapped directly to the pointer (no Animatable mutex →
  // no contention against the down-follow animation, no delayed jumps).
  var dragPointerX: Float by mutableFloatStateOf(0f)
    private set

  // Gates the down-follow animation's writes to dragPointerX. Cleared on any
  // drag/release/cancel so a still-running animate frame can't overwrite a fresh
  // pointer value (cancellation is observed at the next yield, not synchronously).
  private var downFollowEnabled by mutableStateOf(false)

  var isGestureActive by mutableStateOf(false)
    private set

  var deformationTarget by mutableFloatStateOf(0f)
    private set

  private var motionDirection by mutableStateOf(0f)
  private val indicatorDeformer = DirectionalStretchBottomBarIndicatorDeformer()
  private var transitionJob: Job? = null
  private var dragPointerJob: Job? = null

  suspend fun animatePillScale(isPressed: Boolean) {
    if (isPressed) {
      pillScale.animateTo(1.01f, tween(150, easing = EaseOutCubic))
    } else {
      pillScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  fun startTransition(activeTab: Tab) {
    // Skip the LaunchedEffect's initial fire and any redundant calls so the indicator
    // stays settled at its natural bounds (transitionProgress=1) when nothing changed.
    if (lastTransitionedToTab == activeTab && !transitionPending) return
    lastTransitionedToTab = activeTab
    transitionJob?.cancel()
    transitionJob =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        transitionProgress.snapTo(0f)
        transitionPending = false
        coroutineScope {
          Tab.entries.forEach { tab ->
            val target = if (tab == activeTab) 1f else 0f
            launch { tabProgress.getValue(tab).animateTo(target, MainBottomBarPillProgressSpec) }
          }
          launch { transitionProgress.animateTo(1f, MainBottomBarPillProgressSpec) }
        }
      }
  }

  fun beginGesture(
    pressInteraction: PressInteraction.Press,
    currentBaseCenter: Float,
    downX: Float,
  ) {
    isGestureActive = true
    deformationTarget = 0f
    motionDirection = 0f
    dragPointerX = currentBaseCenter
    scope.launch { interactionSource.emit(pressInteraction) }
    dragPointerJob?.cancel()
    downFollowEnabled = true
    dragPointerJob = scope.launch {
      try {
        animate(
          initialValue = currentBaseCenter,
          targetValue = downX,
          animationSpec = tween(MainBottomBarPillDownFollowDurationMillis, easing = EaseOutCubic),
        ) { value, _ ->
          if (downFollowEnabled) dragPointerX = value
        }
      } finally {
        downFollowEnabled = false
      }
    }
  }

  fun followDrag(previousX: Float, targetX: Float, totalWidth: Float): Float {
    val clampedX = targetX.coerceIn(0f, totalWidth)
    val delta = clampedX - previousX
    motionDirection =
      stableIndicatorDirection(
        previousDirection = motionDirection,
        from = previousX,
        to = clampedX,
        minDelta = MainBottomBarPillDirectionMinDeltaPx,
      )
    deformationTarget =
      bottomBarStretchIntensityForDelta(
        delta = delta,
        fullStretchDelta = MainBottomBarPillFullStretchDeltaPx,
      )
    downFollowEnabled = false
    dragPointerJob?.cancel()
    dragPointerX = clampedX
    return clampedX
  }

  fun releaseGesture(
    pressInteraction: PressInteraction.Press,
    snapshotLeft: Float,
    snapshotRight: Float,
    targetTab: Tab,
    onSelectTab: (Tab) -> Unit,
  ) {
    scope.launch { interactionSource.emit(PressInteraction.Release(pressInteraction)) }
    captureSnapshotAndExitGesture(snapshotLeft, snapshotRight)
    onSelectTab(targetTab)
    // Always start the transition directly (covers the same-tab tap case where
    // LaunchedEffect won't fire). Idempotent: if LaunchedEffect also fires for a tab
    // change, the second startTransition call short-circuits via the early return.
    startTransition(targetTab)
  }

  fun cancelGesture(
    pressInteraction: PressInteraction.Press,
    snapshotLeft: Float,
    snapshotRight: Float,
    currentActiveTab: Tab,
  ) {
    scope.launch { interactionSource.emit(PressInteraction.Cancel(pressInteraction)) }
    captureSnapshotAndExitGesture(snapshotLeft, snapshotRight)
    // No tab change → trigger a transition manually so the indicator springs back.
    startTransition(currentActiveTab)
  }

  private fun captureSnapshotAndExitGesture(snapshotLeft: Float, snapshotRight: Float) {
    downFollowEnabled = false
    dragPointerJob?.cancel()
    snapshotPrevLeft = snapshotLeft
    snapshotPrevRight = snapshotRight
    transitionPending = true
    isGestureActive = false
    deformationTarget = 0f
    motionDirection = 0f
  }

  fun indicatorShape(
    naturalLeft: Float,
    naturalRight: Float,
    tabCenters: Map<Tab, Float>,
    tabWidths: Map<Tab, Float>,
    indicatorInsetPx: Float,
    totalWidth: Float,
    deformationIntensity: Float,
  ): BottomBarIndicatorShape? {
    if (totalWidth <= 0f) return null

    val (left, right) =
      when {
        isGestureActive -> {
          // Width tracks the pointer: at tab centers it matches that tab's box width
          // (collapsed or expanded); between centers it interpolates linearly by distance.
          val center = dragPointerX
          val boxWidth = interpolatedBoxWidth(center, tabCenters, tabWidths)
          val width = (boxWidth - indicatorInsetPx * 2f).coerceAtLeast(0f)
          (center - width / 2f) to (center + width / 2f)
        }
        transitionPending -> {
          // Snapshot just captured but the launched startTransition coroutine hasn't
          // executed snapTo(0) yet. Render the snapshot directly to avoid a 1-frame
          // jump to lerp(snapshot, natural, stale 1) = natural.
          snapshotPrevLeft to snapshotPrevRight
        }
        else -> {
          val t = transitionProgress.value
          val l = snapshotPrevLeft + (naturalLeft - snapshotPrevLeft) * t
          val r = snapshotPrevRight + (naturalRight - snapshotPrevRight) * t
          l to r
        }
      }
    val width = (right - left).coerceAtLeast(0f)
    val centerX = (left + right) / 2f

    return indicatorDeformer.deform(
      BottomBarIndicatorDeformerInput(
        centerX = centerX,
        baseWidth = width,
        direction = motionDirection,
        stretchIntensity = deformationIntensity,
        trackStartX = 0f,
        trackEndX = totalWidth,
      )
    )
  }
}

internal fun nearestTab(tabCenters: Map<Tab, Float>, totalWidth: Float, pointerX: Float): Tab {
  val clampedX = pointerX.coerceIn(0f, totalWidth)
  return Tab.entries.minByOrNull { abs((tabCenters[it] ?: 0f) - clampedX) } ?: Tab.entries.first()
}

/**
 * Pointer-driven tab box width for the indicator during a gesture. At each tab's center the
 * indicator matches that tab's box width exactly; between two adjacent tab centers it linearly
 * interpolates based on horizontal distance. Outside the first/last tab center, it clamps to the
 * edge tab's width.
 */
internal fun interpolatedBoxWidth(
  pointerX: Float,
  tabCenters: Map<Tab, Float>,
  tabWidths: Map<Tab, Float>,
): Float {
  val entries = Tab.entries
  if (entries.isEmpty()) return 0f
  val firstTab = entries.first()
  val lastTab = entries.last()
  if (pointerX <= tabCenters.getValue(firstTab)) return tabWidths.getValue(firstTab)
  if (pointerX >= tabCenters.getValue(lastTab)) return tabWidths.getValue(lastTab)
  for (i in 0 until entries.size - 1) {
    val leftTab = entries[i]
    val rightTab = entries[i + 1]
    val leftCenter = tabCenters.getValue(leftTab)
    val rightCenter = tabCenters.getValue(rightTab)
    if (pointerX in leftCenter..rightCenter) {
      val span = rightCenter - leftCenter
      if (span <= 0f) return tabWidths.getValue(leftTab)
      val t = (pointerX - leftCenter) / span
      val leftWidth = tabWidths.getValue(leftTab)
      val rightWidth = tabWidths.getValue(rightTab)
      return leftWidth + (rightWidth - leftWidth) * t
    }
  }
  return tabWidths.getValue(lastTab)
}

/** Indicator bounds that visually cover the pointer-interpolated box at [pointerX]. */
private fun snapshotBoundsAt(
  pointerX: Float,
  tabCenters: Map<Tab, Float>,
  tabWidths: Map<Tab, Float>,
  indicatorInsetPx: Float,
): Pair<Float, Float> {
  val boxWidth = interpolatedBoxWidth(pointerX, tabCenters, tabWidths)
  val indicatorWidth = (boxWidth - indicatorInsetPx * 2f).coerceAtLeast(0f)
  return (pointerX - indicatorWidth / 2f) to (pointerX + indicatorWidth / 2f)
}

internal val MainBottomBarPillIndicatorActiveInset = 4.dp
internal val MainBottomBarPillIndicatorRestingInset = 6.dp
internal const val MainBottomBarPillIndicatorInsetAnimationDurationMillis = 140
private const val MainBottomBarPillProgressDurationMillis = 180
private const val MainBottomBarPillDownFollowDurationMillis = 240
private const val MainBottomBarPillFollowMovementThresholdPx = 1f
private const val MainBottomBarPillDirectionMinDeltaPx = 1.5f
private const val MainBottomBarPillFullStretchDeltaPx = 18f

private val MainBottomBarPillProgressSpec =
  tween<Float>(MainBottomBarPillProgressDurationMillis, easing = EaseOutCubic)
