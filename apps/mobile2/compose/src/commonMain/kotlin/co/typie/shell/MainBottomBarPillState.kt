package co.typie.shell

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.FiniteAnimationSpec
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
import kotlinx.coroutines.launch

@Composable
internal fun MainBottomBarPillEffects(
  state: MainBottomBarPillState,
  trackLayout: BottomBarTrackLayout?,
  currentTab: Tab,
  isPillPressed: Boolean,
) {
  LaunchedEffect(isPillPressed) { state.animatePillScale(isPillPressed) }

  LaunchedEffect(trackLayout, currentTab, state.isGestureActive, state.settlingTargetTab) {
    val layout = trackLayout ?: return@LaunchedEffect
    state.syncSelectedTab(layout = layout, currentTab = currentTab)
  }
}

internal fun Modifier.mainBottomBarPillGestures(
  state: MainBottomBarPillState,
  trackLayout: BottomBarTrackLayout?,
  tabState: TabState,
): Modifier =
  pointerInput(trackLayout, tabState.currentTab, tabState.onSelectTab) {
    val layout = trackLayout ?: return@pointerInput

    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      val pressInteraction = PressInteraction.Press(down.position)
      val trackHeightPx = size.height.toFloat()
      var released = false
      var dragging = false
      var total = Offset.Zero
      var trackedX = layout.clampPointerX(down.position.x)
      var lastClampedPosition =
        Offset(
          x = layout.clampPointerX(down.position.x),
          y = down.position.y.coerceIn(0f, trackHeightPx),
        )

      state.beginGesture(pressInteraction)

      try {
        val downX = layout.clampPointerX(down.position.x)
        val downStartX = state.currentIndicatorCenterX()
        trackedX = downX
        state.animateTowardDownPosition(startX = downStartX, downX = downX)

        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.firstOrNull { it.id == down.id } ?: break

          if (change.changedToUp()) {
            val releaseX = layout.clampPointerX(change.position.x)
            val targetTab = layout.nearestTab(releaseX)
            val targetCenterX = layout.centerFor(targetTab)

            released = true
            state.releaseGesture(
              pressInteraction = pressInteraction,
              releaseX = releaseX,
              targetTab = targetTab,
              targetCenterX = targetCenterX,
              dragging = dragging,
              onSelectTab = tabState.onSelectTab,
            )
            break
          }

          if (change.isConsumed) {
            break
          }

          val clampedPosition =
            Offset(
              x = layout.clampPointerX(change.position.x),
              y = change.position.y.coerceIn(0f, trackHeightPx),
            )
          total += clampedPosition - lastClampedPosition
          lastClampedPosition = clampedPosition
          val pointerX = clampedPosition.x

          if (!dragging) {
            if (abs(total.x) > viewConfiguration.touchSlop) {
              dragging = true
              trackedX = state.currentIndicatorCenterX()
              trackedX =
                state.followDraggedIndicator(
                  previousX = trackedX,
                  targetX = pointerX,
                  layout = layout,
                )
              change.consume()
            } else if (abs(total.y) > viewConfiguration.touchSlop) {
              break
            }
          } else {
            trackedX =
              state.followDraggedIndicator(
                previousX = trackedX,
                targetX = pointerX,
                layout = layout,
              )
            change.consume()
          }
        }
      } finally {
        if (!released) {
          state.cancelGesture(
            pressInteraction = pressInteraction,
            currentTab = tabState.currentTab,
            selectedCenterX = layout.centerFor(tabState.currentTab),
          )
        }
      }
    }
  }

@Composable
internal fun rememberMainBottomBarPillState(): MainBottomBarPillState {
  val scope = rememberCoroutineScope()
  return remember(scope) { MainBottomBarPillState(scope) }
}

@Composable
internal fun rememberMainBottomBarTrackLayout(
  trackWidthPx: Float,
  restingIndicatorInsetPx: Float,
): BottomBarTrackLayout? =
  remember(trackWidthPx, restingIndicatorInsetPx) {
    if (trackWidthPx <= 0f) {
      null
    } else {
      bottomBarTrackLayout(
        trackWidth = trackWidthPx,
        tabCount = Tab.entries.size,
        segmentPadding = restingIndicatorInsetPx,
      )
    }
  }

internal class MainBottomBarPillState(private val scope: CoroutineScope) {
  val interactionSource = MutableInteractionSource()
  val pillScale = Animatable(1f)
  private val animatedIndicatorCenterX = Animatable(0f)
  private val indicatorDeformer = DirectionalStretchBottomBarIndicatorDeformer()

  var trackWidthPx by mutableStateOf(0f)
  var isGestureActive by mutableStateOf(false)
    private set

  var settlingTargetTab by mutableStateOf<Tab?>(null)
    private set

  var deformationTarget by mutableFloatStateOf(0f)
    private set

  private var hasInitializedIndicator = false
  private var isIndicatorDraggedDirectly by mutableStateOf(false)
  private var motionDirection by mutableStateOf(0f)
  private var directIndicatorCenterX by mutableFloatStateOf(0f)
  private var indicatorAnimationJob: Job? = null

  suspend fun animatePillScale(isPressed: Boolean) {
    if (isPressed) {
      pillScale.animateTo(1.01f, tween(150, easing = EaseOutCubic))
    } else {
      pillScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  suspend fun syncSelectedTab(layout: BottomBarTrackLayout, currentTab: Tab) {
    val selectedCenterX = layout.centerFor(currentTab)

    if (!hasInitializedIndicator) {
      directIndicatorCenterX = selectedCenterX
      animatedIndicatorCenterX.snapTo(selectedCenterX)
      motionDirection = 0f
      hasInitializedIndicator = true
      return
    }

    if (isGestureActive || settlingTargetTab != null) {
      return
    }

    val currentCenterX = currentIndicatorCenterX()
    isIndicatorDraggedDirectly = false
    deformationTarget = 0f
    motionDirection = directionFor(from = currentCenterX, to = selectedCenterX)
    animateIndicatorTo(
      startX = currentCenterX,
      targetX = selectedCenterX,
      kind = MainBottomBarPillIndicatorAnimationKind.ExternalSync,
    )
  }

  fun currentIndicatorCenterX(): Float =
    if (isIndicatorDraggedDirectly) directIndicatorCenterX else animatedIndicatorCenterX.value

  fun beginGesture(pressInteraction: PressInteraction.Press) {
    isGestureActive = true
    isIndicatorDraggedDirectly = false
    settlingTargetTab = null
    deformationTarget = 0f
    emitInteraction(pressInteraction)
  }

  fun animateTowardDownPosition(startX: Float, downX: Float) {
    motionDirection = directionFor(from = startX, to = downX)
    animateIndicatorTo(
      startX = startX,
      targetX = downX,
      kind = MainBottomBarPillIndicatorAnimationKind.DownFollow,
    )
  }

  fun followDraggedIndicator(
    previousX: Float,
    targetX: Float,
    layout: BottomBarTrackLayout,
  ): Float {
    val clampedX = layout.clampPointerX(targetX)
    val delta = clampedX - previousX
    motionDirection =
      stableIndicatorDirection(
        previousDirection = motionDirection,
        from = previousX,
        to = clampedX,
        minDelta = 1.5f,
      )
    deformationTarget = bottomBarStretchIntensityForDelta(delta = delta, fullStretchDelta = 18f)
    directIndicatorCenterX = clampedX
    isIndicatorDraggedDirectly = true
    return clampedX
  }

  fun releaseGesture(
    pressInteraction: PressInteraction.Press,
    releaseX: Float,
    targetTab: Tab,
    targetCenterX: Float,
    dragging: Boolean,
    onSelectTab: (Tab) -> Unit,
  ) {
    emitInteraction(PressInteraction.Release(pressInteraction))

    isGestureActive = false
    val releaseStartX = currentIndicatorCenterX()
    isIndicatorDraggedDirectly = false
    settlingTargetTab = targetTab
    deformationTarget = 0f
    onSelectTab(targetTab)

    if (dragging) {
      animateIndicatorReleaseToTab(
        startX = releaseStartX,
        releaseX = releaseX,
        targetX = targetCenterX,
      )
    } else {
      motionDirection = directionFor(from = releaseStartX, to = targetCenterX)
      animateIndicatorTo(
        startX = releaseStartX,
        targetX = targetCenterX,
        kind = MainBottomBarPillIndicatorAnimationKind.Snap,
      )
    }
  }

  fun cancelGesture(
    pressInteraction: PressInteraction.Press,
    currentTab: Tab,
    selectedCenterX: Float,
  ) {
    emitInteraction(PressInteraction.Cancel(pressInteraction))

    isGestureActive = false
    val cancelStartX = currentIndicatorCenterX()
    isIndicatorDraggedDirectly = false
    settlingTargetTab = currentTab
    deformationTarget = 0f
    motionDirection = directionFor(from = cancelStartX, to = selectedCenterX)
    animateIndicatorTo(
      startX = cancelStartX,
      targetX = selectedCenterX,
      kind = MainBottomBarPillIndicatorAnimationKind.Snap,
    )
  }

  fun indicatorShape(
    trackLayout: BottomBarTrackLayout?,
    visualIndicatorInsetPx: Float,
    deformationIntensity: Float,
  ): BottomBarIndicatorShape? = trackLayout?.let { layout ->
    val visualIndicatorBaseWidth =
      ((layout.trackWidth / Tab.entries.size.toFloat()) - visualIndicatorInsetPx * 2f)
        .coerceAtLeast(0f)

    indicatorDeformer.deform(
      BottomBarIndicatorDeformerInput(
        centerX = currentIndicatorCenterX(),
        baseWidth = visualIndicatorBaseWidth,
        direction = motionDirection,
        stretchIntensity = deformationIntensity,
        trackStartX = 0f,
        trackEndX = layout.trackWidth,
      )
    )
  }

  private fun emitInteraction(interaction: PressInteraction) {
    scope.launch { interactionSource.emit(interaction) }
  }

  private fun animateIndicatorTo(
    startX: Float,
    targetX: Float,
    kind: MainBottomBarPillIndicatorAnimationKind,
  ) {
    indicatorAnimationJob?.cancel()
    indicatorAnimationJob =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        animatedIndicatorCenterX.snapTo(startX)
        animatedIndicatorCenterX.animateTo(targetX, kind.animationSpec())
        motionDirection = 0f
        if (settlingTargetTab != null) {
          settlingTargetTab = null
        }
      }
  }

  private fun animateIndicatorReleaseToTab(startX: Float, releaseX: Float, targetX: Float) {
    indicatorAnimationJob?.cancel()
    indicatorAnimationJob =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        animatedIndicatorCenterX.snapTo(startX)

        if (abs(releaseX - startX) > 0.5f) {
          motionDirection = directionFor(from = startX, to = releaseX)
          animatedIndicatorCenterX.animateTo(
            releaseX,
            MainBottomBarPillIndicatorAnimationKind.DownFollow.animationSpec(),
          )
        }

        if (abs(targetX - animatedIndicatorCenterX.value) > 0.5f) {
          motionDirection = directionFor(from = animatedIndicatorCenterX.value, to = targetX)
          animatedIndicatorCenterX.animateTo(
            targetX,
            MainBottomBarPillIndicatorAnimationKind.Snap.animationSpec(),
          )
        }

        motionDirection = 0f
        if (settlingTargetTab != null) {
          settlingTargetTab = null
        }
      }
  }
}

internal val MainBottomBarPillIndicatorActiveInset = 2.dp
internal val MainBottomBarPillIndicatorRestingInset = 4.dp
private const val MainBottomBarPillDownFollowDurationMillis = 240
internal const val MainBottomBarPillIndicatorInsetAnimationDurationMillis = 140

private enum class MainBottomBarPillIndicatorAnimationKind {
  DownFollow,
  Snap,
  ExternalSync;

  fun animationSpec(): FiniteAnimationSpec<Float> =
    when (this) {
      DownFollow -> tween(MainBottomBarPillDownFollowDurationMillis, easing = EaseOutCubic)
      Snap -> spring(dampingRatio = 0.8f, stiffness = 600f)
      ExternalSync -> tween(200, easing = EaseOutCubic)
    }
}

private fun directionFor(from: Float, to: Float): Float =
  when {
    to > from -> 1f
    to < from -> -1f
    else -> 0f
  }
