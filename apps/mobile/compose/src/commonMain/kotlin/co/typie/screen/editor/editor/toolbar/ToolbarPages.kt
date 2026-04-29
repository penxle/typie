package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.ScrollableDefaults
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.contextual.editorImageToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.rememberTextToolbarPage
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.hazeEffect
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

@Composable
internal fun rememberEditorToolbarPages(): List<EditorToolbarPage> {
  val textToolbarPage = rememberTextToolbarPage()
  return remember(textToolbarPage) {
    listOf(editorMainToolbarPage(), textToolbarPage, editorImageToolbarPage())
  }
}

@Composable
internal fun EditorToolbarPages(
  pages: List<EditorToolbarPage>,
  editorFocused: Boolean,
  activeBottomPanel: EditorToolbarBottomPanelKey?,
  fixedAction: ToolbarFixedAction,
  onEditorInputRequest: () -> Unit,
  onKeyboardDismissRequest: () -> Unit,
  onBottomPanelToggle: (EditorToolbarBottomPanelKey) -> Unit,
  modifier: Modifier = Modifier,
) {
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val hazeState = LocalHazeState.current
  val pagerState = rememberToolbarPagerState()

  BoxWithConstraints(modifier = modifier.height(ToolbarStackHeight)) {
    val pageCount = pages.size.coerceAtLeast(1)
    val lastPageIndex = pageCount - 1
    val pageDistance = with(density) { maxWidth.roundToPx().coerceAtLeast(0) }.toFloat()
    val hardStopOverscrollLimitPx = with(density) { ToolbarHardStopOverscrollLimit.toPx() }
    val hardStopActivationEpsilonPx = with(density) { ToolbarHardStopActivationEpsilon.toPx() }
    val pageScrollRanges = pages.map { it.scrollState?.maxValue ?: 0 }
    val pageMetrics =
      remember(pageDistance, pageScrollRanges) {
        ToolbarPagerMetrics(pageDistance = pageDistance, scrollRanges = pageScrollRanges)
      }
    val pageProgress = pageMetrics.progressFor(pagerState.scrollPosition)
    val indicatorProgress = pagerState.indicatorDragProgress ?: pageProgress
    val currentPageIndex = pageMetrics.pageIndexForPosition(pagerState.scrollPosition)
    val scrollableState = rememberScrollableState { delta ->
      val currentPosition = pagerState.scrollPosition
      val gestureStartPosition =
        pagerState.scrollGestureStartPosition
          ?: currentPosition.also { pagerState.scrollGestureStartPosition = it }
      val proposedPosition = (currentPosition - delta).coerceIn(0f, pageMetrics.maxPosition)
      val scrollResult =
        pageMetrics.applyHardStop(
          currentPosition = currentPosition,
          proposedPosition = proposedPosition,
          hardStop = pagerState.activeHardStop,
          gestureStartPosition = gestureStartPosition,
          activationEpsilon = hardStopActivationEpsilonPx,
        )
      val nextPosition = scrollResult.position
      val bounceHardStopDuringDecay =
        pagerState.decayFlingInProgress &&
          scrollResult.rejectedDelta != 0f &&
          !pagerState.decayHardStopBounceStarted
      val consumed =
        if (scrollResult.rejectedDelta != 0f) {
          if (pagerState.decayFlingInProgress) currentPosition - nextPosition else delta
        } else {
          currentPosition - nextPosition
        }
      val nextVisualOffset =
        if (scrollResult.rejectedDelta != 0f) {
          if (pagerState.decayFlingInProgress && pagerState.decayHardStopBounceStarted) {
            pagerState.hardStopVisualOffset.value
          } else {
            (pagerState.hardStopVisualOffset.value -
                scrollResult.rejectedDelta * ToolbarHardStopOverscrollResistance)
              .coerceIn(-hardStopOverscrollLimitPx, hardStopOverscrollLimitPx)
          }
        } else if (scrollResult.hardStop == null) {
          0f
        } else {
          pagerState.hardStopVisualOffset.value
        }

      pagerState.activeHardStop = scrollResult.hardStop
      if (consumed != 0f) {
        pagerState.scrollPosition = nextPosition
        if (pagerState.scrollPositionAnimation.isRunning) {
          scope.launch {
            pagerState.scrollPositionAnimation.stop()
            pagerState.scrollPositionAnimation.snapTo(nextPosition)
          }
        }
      }
      if (scrollResult.rejectedDelta == 0f) {
        pagerState.decayHardStopBounceStarted = false
      }
      if (bounceHardStopDuringDecay) {
        pagerState.decayHardStopBounceStarted = true
      }
      if (nextVisualOffset != pagerState.hardStopVisualOffset.value || bounceHardStopDuringDecay) {
        scope.launch {
          pagerState.hardStopVisualOffset.stop()
          if (nextVisualOffset != pagerState.hardStopVisualOffset.value) {
            pagerState.hardStopVisualOffset.snapTo(nextVisualOffset)
          }
          if (bounceHardStopDuringDecay) {
            pagerState.hardStopVisualOffset.animateTo(0f, ToolbarHardStopOverscrollSpring)
            pagerState.decayHardStopBounceStarted = false
          }
        }
      }
      consumed
    }

    LaunchedEffect(scrollableState) {
      snapshotFlow { scrollableState.isScrollInProgress }
        .distinctUntilChanged()
        .collect { inProgress ->
          if (!inProgress) {
            delay(ToolbarScrollGestureIdleResetMillis)
            if (!scrollableState.isScrollInProgress && !pagerState.pointerScrollGestureActive) {
              pagerState.scrollGestureStartPosition = null
            }
          }
        }
    }

    LaunchedEffect(pageCount, pageMetrics) {
      val coercedPage = pagerState.settledPageIndex.coerceIn(0, lastPageIndex)
      val coercedPosition = pagerState.scrollPosition.coerceIn(0f, pageMetrics.maxPosition)
      if (coercedPosition != pagerState.scrollPosition) {
        pagerState.scrollPosition = coercedPosition
        pagerState.scrollPositionAnimation.snapTo(coercedPosition)
      }
      pagerState.settledPageIndex = coercedPage
    }

    LaunchedEffect(pages, pageMetrics) {
      snapshotFlow {
          pages.mapIndexedNotNull { index, page ->
            val scrollState = page.scrollState ?: return@mapIndexedNotNull null
            val target =
              pageMetrics.internalScrollFor(index, pagerState.scrollPosition).roundToInt()
            scrollState to target.coerceIn(0, scrollState.maxValue)
          }
        }
        .collect { scrollTargets ->
          scrollTargets.forEach { (scrollState, target) ->
            if (scrollState.value != target) {
              scrollState.scrollTo(target)
            }
          }
        }
    }

    LaunchedEffect(pageMetrics) {
      var initialized = false
      snapshotFlow { pageMetrics.isPageTransitionPosition(pagerState.scrollPosition) }
        .distinctUntilChanged()
        .collect { transitioning ->
          pagerState.indicatorPageTransitioning = transitioning
          if (initialized && transitioning) {
            pagerState.indicatorPulse++
          } else {
            initialized = true
          }
        }
    }

    LaunchedEffect(pagerState.hardStopVisualOffset) {
      var initialized = false
      snapshotFlow { abs(pagerState.hardStopVisualOffset.value) > ToolbarSnapPositionEpsilon }
        .distinctUntilChanged()
        .collect { hardStopping ->
          if (initialized && hardStopping) {
            pagerState.indicatorPulse++
          } else {
            initialized = true
          }
        }
    }

    val indicatorHeldVisible =
      pagerState.indicatorInteracting || pagerState.indicatorPageTransitioning
    LaunchedEffect(pagerState.indicatorPulse, indicatorHeldVisible) {
      if (pagerState.indicatorPulse == 0 && !indicatorHeldVisible) {
        pagerState.indicatorVisible = false
        return@LaunchedEffect
      }
      pagerState.indicatorVisible = true
      if (!indicatorHeldVisible) {
        delay(ToolbarIndicatorVisibleMillis)
        pagerState.indicatorVisible = false
      }
    }

    suspend fun animateScrollPositionTo(targetPosition: Float, initialVelocity: Float = 0f) {
      pagerState.scrollPositionAnimation.stop()
      pagerState.scrollPositionAnimation.snapTo(pagerState.scrollPosition)
      pagerState.scrollPositionAnimation.animateTo(
        targetPosition,
        initialVelocity = initialVelocity,
      ) {
        pagerState.scrollPosition = value
      }
    }

    fun navigateToPageIndex(pageIndex: Int) {
      scope.launch {
        val targetPageIndex = pageIndex.coerceIn(0, lastPageIndex)
        if (targetPageIndex != pagerState.settledPageIndex) {
          onEditorInputRequest()
        }
        pagerState.activeHardStop = null
        pagerState.hardStopVisualOffset.snapTo(0f)
        val targetPosition =
          pageMetrics.positionForPageEntry(
            pageIndex = targetPageIndex,
            fromPageIndex = currentPageIndex,
          )
        animateScrollPositionTo(targetPosition = targetPosition)
        pagerState.settledPageIndex = targetPageIndex
      }
    }

    fun navigateToPage(page: EditorToolbarPageKey) {
      val pageIndex = pages.indexOfFirst { it.key == page }
      if (pageIndex >= 0) {
        navigateToPageIndex(pageIndex)
      }
    }

    suspend fun settlePages(velocity: Float = 0f) {
      val snapPosition =
        pageMetrics.snapPosition(pagerState.scrollPosition, velocity, pagerState.activeHardStop)
      val snapPage = pageMetrics.pageIndexForPosition(snapPosition)
      if (snapPage != pagerState.settledPageIndex) {
        onEditorInputRequest()
      }
      animateScrollPositionTo(targetPosition = snapPosition, initialVelocity = -velocity)
      pagerState.settledPageIndex = snapPage
      pagerState.activeHardStop = null
      if (pagerState.hardStopVisualOffset.value != 0f) {
        pagerState.hardStopVisualOffset.animateTo(0f, ToolbarHardStopOverscrollSpring)
      }
    }

    val defaultFlingBehavior = ScrollableDefaults.flingBehavior()
    val flingBehavior =
      remember(pageMetrics, pagerState.activeHardStop, onEditorInputRequest, defaultFlingBehavior) {
        object : FlingBehavior {
          override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
            val remainingVelocity =
              if (
                pageMetrics.decaysFlingWithinInternalScroll(
                  position = pagerState.scrollPosition,
                  velocity = initialVelocity,
                )
              ) {
                try {
                  pagerState.decayHardStopBounceStarted = false
                  pagerState.decayFlingInProgress = true
                  with(defaultFlingBehavior) { performFling(initialVelocity) }
                } finally {
                  pagerState.decayFlingInProgress = false
                }
              } else {
                initialVelocity
              }
            settlePages(remainingVelocity)
            return 0f
          }
        }
      }

    val indicatorAlpha by
      animateFloatAsState(
        targetValue = if (pagerState.indicatorVisible || indicatorHeldVisible) 1f else 0f,
        animationSpec = tween(ToolbarIndicatorFadeMillis),
        label = "editor-toolbar-indicator-alpha",
      )

    if (pageCount > 1) {
      EditorToolbarIndicatorPill(
        pages = pages,
        pageProgress = indicatorProgress,
        animateBackground = pagerState.indicatorInteracting && !pagerState.indicatorDragging,
        currentPageIndex = currentPageIndex,
        modifier =
          Modifier.align(Alignment.TopCenter)
            .alpha(indicatorAlpha)
            .then(
              if (indicatorAlpha > 0.01f) {
                Modifier.toolbarIndicatorGestures(
                    pageCount = pageCount,
                    currentPageIndex = currentPageIndex,
                    onIndicatorProgress = { progress ->
                      pagerState.indicatorDragProgress = progress
                    },
                    onIndicatorDraggingChange = { dragging ->
                      pagerState.indicatorDragging = dragging
                    },
                    onPageSelected = { page -> navigateToPageIndex(page) },
                    onInteractionActiveChange = { active ->
                      pagerState.indicatorInteracting = active
                      if (!active) {
                        pagerState.indicatorDragProgress = null
                        pagerState.indicatorDragging = false
                      }
                      pagerState.indicatorPulse++
                    },
                  )
                  .preserveEditorFocusOnToolbarInteraction()
              } else {
                Modifier
              }
            ),
      )
    }

    InteractionScope {
      val toolbarInteractionSource =
        LocalInteractionSource.current ?: remember { MutableInteractionSource() }
      val toolbarSurfaceColor = AppTheme.colors.surfaceDefault
      Box(
        modifier =
          Modifier.align(Alignment.BottomCenter)
            .fillMaxWidth()
            .height(ToolbarHeight)
            .shadow(AppTheme.shadows.sm, ToolbarCapsuleShape)
            .pressScale(ToolbarCapsulePressedScale)
            .clip(ToolbarCapsuleShape)
            .hazeEffect(hazeState) {
              backgroundColor = toolbarSurfaceColor
              blurRadius = ToolbarBackdropBlurRadius
            }
            .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarCapsuleShape)
      ) {
        EditorToolbarSurfaceBackground(shape = ToolbarCapsuleShape)

        Box(
          modifier =
            Modifier.matchParentSize()
              .clipToBounds()
              .emitPressInteractions(toolbarInteractionSource)
              .trackToolbarScrollGestureStart(
                onStart = {
                  pagerState.pointerScrollGestureActive = true
                  pagerState.scrollGestureStartPosition = pagerState.scrollPosition
                },
                onEnd = { pagerState.pointerScrollGestureActive = false },
              )
              .scrollable(
                state = scrollableState,
                orientation = Orientation.Horizontal,
                enabled = pageDistance > 0f && pageCount > 1,
                flingBehavior = flingBehavior,
                interactionSource = toolbarInteractionSource,
              )
              .preserveEditorFocusOnToolbarInteraction()
        ) {
          Box(
            modifier =
              Modifier.fillMaxSize().graphicsLayer {
                translationX = pagerState.hardStopVisualOffset.value
              }
          ) {
            pages.forEachIndexed { index, page ->
              val pageScope =
                EditorToolbarPageScope(
                  activeBottomPanel = activeBottomPanel,
                  hasNextPage = index < lastPageIndex,
                  navigateToPage = ::navigateToPage,
                  toggleBottomPanel = onBottomPanelToggle,
                )

              Box(
                modifier =
                  Modifier.fillMaxSize().offset {
                    val pageOffset = pageMetrics.pageOffsetFor(index, pagerState.scrollPosition)
                    IntOffset(x = pageOffset.roundToInt(), y = 0)
                  }
              ) {
                page.content(pageScope)
              }
            }
          }
        }

        InteractionScope {
          EditorToolbarIconButton(
            icon =
              when (fixedAction) {
                ToolbarFixedAction.ClosePanel -> Lucide.CircleX
                ToolbarFixedAction.HideToolbar -> Lucide.ChevronDown
                ToolbarFixedAction.DismissInput -> Lucide.KeyboardOff
              },
            contentDescription =
              when (fixedAction) {
                ToolbarFixedAction.ClosePanel -> "하단 패널 닫기"
                ToolbarFixedAction.HideToolbar -> "툴바 숨기기"
                ToolbarFixedAction.DismissInput -> if (editorFocused) "에디터 포커스 해제" else "키보드 닫기"
              },
            onClick = onKeyboardDismissRequest,
            shape = ToolbarFixedActionShape,
            fixedActionSurface = true,
            inheritInteractionSource = true,
            crossfadeIcon = true,
            modifier =
              Modifier.align(Alignment.CenterEnd)
                .width(ToolbarFixedActionWidth)
                .fillMaxHeight()
                .padding(ToolbarFixedActionPadding)
                .pressScale(ToolbarFixedActionPressedScale),
          )
        }
      }
    }
  }
}
