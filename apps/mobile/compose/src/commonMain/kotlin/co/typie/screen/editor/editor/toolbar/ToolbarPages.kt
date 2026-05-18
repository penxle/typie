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
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlainNode
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.contextual.editorArchivedToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorBlockquoteToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorCalloutToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorEmbedToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorFileToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorFoldToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorHorizontalRuleToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorImageToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorListToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.editorTableToolbarPage
import co.typie.screen.editor.editor.toolbar.contextual.rememberTextToolbarPage
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.hazeEffect
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

@Composable
internal fun rememberEditorToolbarPages(
  toolbarContext: EditorToolbarContext
): List<EditorToolbarPage> {
  val textToolbarPage = rememberTextToolbarPage()
  return remember(toolbarContext, textToolbarPage) {
    toolbarContext.pageKeys.map { key ->
      when (key) {
        EditorToolbarPageKey.Main ->
          editorMainToolbarPage(hasTextPage = EditorToolbarPageKey.Text in toolbarContext.pageKeys)
        EditorToolbarPageKey.Text -> textToolbarPage
        EditorToolbarPageKey.Image ->
          editorImageToolbarPage(
            image = toolbarContext.selectedNode as? PlainNode.Image,
            nodeId = toolbarContext.selectedNodeId,
          )
        EditorToolbarPageKey.File ->
          editorFileToolbarPage(
            file = toolbarContext.selectedNode as? PlainNode.File,
            nodeId = toolbarContext.selectedNodeId,
          )
        EditorToolbarPageKey.Embed -> editorEmbedToolbarPage()
        EditorToolbarPageKey.Archived -> editorArchivedToolbarPage()
        EditorToolbarPageKey.HorizontalRule -> editorHorizontalRuleToolbarPage()
        EditorToolbarPageKey.List -> editorListToolbarPage(toolbarContext.listMode)
        EditorToolbarPageKey.Blockquote -> editorBlockquoteToolbarPage()
        EditorToolbarPageKey.Callout -> editorCalloutToolbarPage()
        EditorToolbarPageKey.Fold -> editorFoldToolbarPage()
        EditorToolbarPageKey.Table -> editorTableToolbarPage(toolbarContext.tableMode)
      }
    }
  }
}

@Composable
internal fun EditorToolbarPages(
  pages: List<EditorToolbarPage>,
  commandScope: CoroutineScope,
  pagerState: ToolbarPagerState = rememberToolbarPagerState(),
  autoTargetPageKey: EditorToolbarPageKey? = null,
  autoTargetRevision: Long = 0L,
  editorFocused: Boolean,
  activeBottomPanel: EditorToolbarBottomPanelKey?,
  fixedAction: ToolbarFixedAction,
  onEditorInputRequest: () -> Unit,
  onKeyboardDismissRequest: () -> Unit,
  onBottomPanelToggle: (EditorToolbarBottomPanelKey) -> Unit,
  onEditorMessage: (Message) -> Unit = {},
  modifier: Modifier = Modifier,
) {
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val hazeState = LocalHazeState.current

  BoxWithConstraints(modifier = modifier.height(ToolbarStackHeight)) {
    val pageKeys = pages.map { it.key }
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
    val validAutoTargetPageKey = autoTargetPageKey?.takeIf { target -> target in pageKeys }
    val pageKeysChangedInFrame =
      pagerState.previousPageKeys?.let { previousPageKeys -> previousPageKeys != pageKeys } == true
    val retainedPageIndex = pages.indexOfFirst { page -> page.key == pagerState.settledPageKey }
    val retainedPagePosition =
      if (
        pageKeysChangedInFrame &&
          retainedPageIndex >= 0 &&
          (validAutoTargetPageKey == null || validAutoTargetPageKey == pagerState.settledPageKey)
      ) {
        val retainedScrollState = pages[retainedPageIndex].scrollState
        val retainedInternalScroll =
          retainedScrollState?.value?.coerceIn(0, retainedScrollState.maxValue) ?: 0
        pageMetrics.positionForPageEntry(pageIndex = retainedPageIndex) + retainedInternalScroll
      } else {
        null
      }
    val visualScrollPosition = retainedPagePosition ?: pagerState.scrollPosition
    val pageProgress = pageMetrics.progressFor(visualScrollPosition)
    val indicatorProgress = pagerState.indicatorDragProgress ?: pageProgress
    val currentPageIndex = pageMetrics.pageIndexForPosition(visualScrollPosition)
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

    LaunchedEffect(Unit) { pagerState.indicatorPulse++ }

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

    LaunchedEffect(pageMetrics) {
      val coercedPosition = pagerState.scrollPosition.coerceIn(0f, pageMetrics.maxPosition)
      if (coercedPosition != pagerState.scrollPosition) {
        pagerState.scrollPosition = coercedPosition
        pagerState.scrollPositionAnimation.snapTo(coercedPosition)
      }
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

    suspend fun snapScrollPositionTo(targetPosition: Float) {
      pagerState.scrollPositionAnimation.stop()
      pagerState.scrollPositionAnimation.snapTo(targetPosition)
      pagerState.scrollPosition = targetPosition
    }

    suspend fun moveToPageKey(
      pageKey: EditorToolbarPageKey,
      animate: Boolean,
      resetInternalScroll: Boolean,
    ) {
      val targetPageIndex = pages.indexOfFirst { it.key == pageKey }
      if (targetPageIndex < 0) {
        return
      }
      val targetPosition =
        if (resetInternalScroll) {
          pageMetrics.positionForPageEntry(pageIndex = targetPageIndex)
        } else if (pageKey == pagerState.settledPageKey) {
          val targetScrollState = pages[targetPageIndex].scrollState
          val targetInternalScroll =
            targetScrollState?.value?.coerceIn(0, targetScrollState.maxValue)?.toFloat() ?: 0f
          pageMetrics.positionForPageEntry(pageIndex = targetPageIndex) + targetInternalScroll
        } else {
          pageMetrics.positionForPageEntry(
            pageIndex = targetPageIndex,
            fromPageIndex = pageMetrics.pageIndexForPosition(pagerState.scrollPosition),
          )
        }
      if (
        pageKey == pagerState.settledPageKey &&
          abs(targetPosition - pagerState.scrollPosition) <= ToolbarSnapPositionEpsilon
      ) {
        return
      }

      pagerState.activeHardStop = null
      pagerState.hardStopVisualOffset.snapTo(0f)
      if (animate) {
        animateScrollPositionTo(targetPosition = targetPosition)
      } else {
        snapScrollPositionTo(targetPosition)
      }
      pagerState.settledPageKey = pageKey
    }

    fun navigateToPageIndex(pageIndex: Int) {
      scope.launch {
        val targetPageIndex = pageIndex.coerceIn(0, lastPageIndex)
        if (pages[targetPageIndex].key != pagerState.settledPageKey) {
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
        pagerState.settledPageKey = pages[targetPageIndex].key
        pagerState.recordManualPageKey(pages[targetPageIndex].key)
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
      val snapPageKey = pages.getOrNull(snapPage)?.key ?: EditorToolbarPageKey.Main
      if (snapPageKey != pagerState.settledPageKey) {
        onEditorInputRequest()
      }
      animateScrollPositionTo(targetPosition = snapPosition, initialVelocity = -velocity)
      pagerState.settledPageKey = snapPageKey
      pagerState.recordManualPageKey(pagerState.settledPageKey)
      pagerState.activeHardStop = null
      if (pagerState.hardStopVisualOffset.value != 0f) {
        pagerState.hardStopVisualOffset.animateTo(0f, ToolbarHardStopOverscrollSpring)
      }
    }

    LaunchedEffect(pageKeys, validAutoTargetPageKey, autoTargetRevision, pageMetrics) {
      val previousPageKeys = pagerState.previousPageKeys
      val initialized = previousPageKeys != null
      val pageKeysChanged = previousPageKeys != null && previousPageKeys != pageKeys
      pagerState.previousPageKeys = pageKeys
      if (pageKeysChanged) {
        pagerState.indicatorPulse++
      }
      val previousAutoReturnPageKey = pagerState.capturedAutoReturnPageKey
      val policyAutoReturnPageKey =
        if (validAutoTargetPageKey == null) {
          previousAutoReturnPageKey
        } else {
          null
        }
      if (validAutoTargetPageKey != null && previousAutoReturnPageKey == null) {
        pagerState.capturedAutoReturnPageKey = pagerState.recentManualPageKeys.firstOrNull()
      }
      snapshotFlow {
          scrollableState.isScrollInProgress ||
            pagerState.pointerScrollGestureActive ||
            pagerState.decayFlingInProgress
        }
        .first { inProgress -> !inProgress }
      val targetPageKey =
        validAutoTargetPageKey
          ?: policyAutoReturnPageKey?.takeIf { target -> target in pageKeys }
          ?: pagerState.recentManualPageKeys.firstOrNull { key -> key in pageKeys }
          ?: EditorToolbarPageKey.Main
      moveToPageKey(
        pageKey = targetPageKey,
        animate = initialized && targetPageKey != pagerState.settledPageKey,
        resetInternalScroll = targetPageKey != pagerState.settledPageKey,
      )
      if (validAutoTargetPageKey == null && previousAutoReturnPageKey != null) {
        pagerState.capturedAutoReturnPageKey = null
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
                  commandScope = commandScope,
                  hasNextPage = index < lastPageIndex,
                  navigateToPage = ::navigateToPage,
                  toggleBottomPanel = onBottomPanelToggle,
                  sendMessage = onEditorMessage,
                )

              Box(
                modifier =
                  Modifier.fillMaxSize().offset {
                    val pageOffset = pageMetrics.pageOffsetFor(index, visualScrollPosition)
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
