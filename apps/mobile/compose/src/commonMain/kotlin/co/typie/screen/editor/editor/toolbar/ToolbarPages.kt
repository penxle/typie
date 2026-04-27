package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollScope
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
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
import co.typie.ui.theme.shadow
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.delay
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
  onEditorInputRequest: () -> Unit,
  onKeyboardDismissRequest: () -> Unit,
  onBottomPanelToggle: (EditorToolbarBottomPanelKey) -> Unit,
  modifier: Modifier = Modifier,
) {
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val scrollPosition = remember { Animatable(0f) }
  val hardStopVisualOffset = remember { Animatable(0f) }
  var indicatorVisible by remember { mutableStateOf(false) }
  var indicatorInteracting by remember { mutableStateOf(false) }
  var indicatorDragging by remember { mutableStateOf(false) }
  var indicatorPulse by remember { mutableIntStateOf(0) }
  var indicatorDragProgress by remember { mutableStateOf<Float?>(null) }
  var settledPageIndex by remember { mutableIntStateOf(0) }
  var activeHardStop by remember { mutableStateOf<ToolbarHardStop?>(null) }

  BoxWithConstraints(modifier = modifier.height(ToolbarStackHeight)) {
    val pageCount = pages.size.coerceAtLeast(1)
    val lastPageIndex = pageCount - 1
    val pageDistance = with(density) { maxWidth.roundToPx().coerceAtLeast(0) }.toFloat()
    val hardStopOverscrollLimitPx = with(density) { ToolbarHardStopOverscrollLimit.toPx() }
    val pageScrollRanges = pages.map { it.scrollState?.maxValue ?: 0 }
    val pageMetrics =
      remember(pageDistance, pageScrollRanges) {
        ToolbarPageMetrics(pageDistance = pageDistance, scrollRanges = pageScrollRanges)
      }
    val pageProgress = pageMetrics.progressFor(scrollPosition.value)
    val indicatorProgress = indicatorDragProgress ?: pageProgress
    val currentPageIndex = pageMetrics.pageIndexForPosition(scrollPosition.value)
    val scrollableState = rememberScrollableState { delta ->
      val currentPosition = scrollPosition.value
      val proposedPosition = (currentPosition - delta).coerceIn(0f, pageMetrics.maxPosition)
      val scrollResult =
        pageMetrics.applyHardStop(
          currentPosition = currentPosition,
          proposedPosition = proposedPosition,
          hardStop = activeHardStop,
        )
      val nextPosition = scrollResult.position
      val consumed = currentPosition - nextPosition
      val nextVisualOffset =
        if (scrollResult.rejectedDelta != 0f) {
          (hardStopVisualOffset.value -
              scrollResult.rejectedDelta * ToolbarHardStopOverscrollResistance)
            .coerceIn(-hardStopOverscrollLimitPx, hardStopOverscrollLimitPx)
        } else if (scrollResult.hardStop == null) {
          0f
        } else {
          hardStopVisualOffset.value
        }

      activeHardStop = scrollResult.hardStop
      if (consumed != 0f) {
        scope.launch {
          scrollPosition.stop()
          scrollPosition.snapTo(nextPosition)
        }
      }
      if (nextVisualOffset != hardStopVisualOffset.value) {
        scope.launch {
          hardStopVisualOffset.stop()
          hardStopVisualOffset.snapTo(nextVisualOffset)
        }
      }
      consumed
    }

    LaunchedEffect(pageCount, pageMetrics) {
      val coercedPage = settledPageIndex.coerceIn(0, lastPageIndex)
      val coercedPosition = scrollPosition.value.coerceIn(0f, pageMetrics.maxPosition)
      if (coercedPosition != scrollPosition.value) {
        scrollPosition.snapTo(coercedPosition)
      }
      settledPageIndex = coercedPage
    }

    LaunchedEffect(pages, pageMetrics) {
      snapshotFlow {
          pages.mapIndexedNotNull { index, page ->
            val scrollState = page.scrollState ?: return@mapIndexedNotNull null
            val target = pageMetrics.internalScrollFor(index, scrollPosition.value).roundToInt()
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

    LaunchedEffect(scrollPosition) {
      var initialized = false
      snapshotFlow { scrollPosition.value.roundToInt() }
        .collect {
          if (initialized) {
            indicatorPulse++
          } else {
            initialized = true
          }
        }
    }

    LaunchedEffect(indicatorPulse, indicatorInteracting) {
      if (indicatorPulse == 0 && !indicatorInteracting) {
        indicatorVisible = false
        return@LaunchedEffect
      }
      indicatorVisible = true
      if (!indicatorInteracting) {
        delay(ToolbarIndicatorVisibleMillis)
        indicatorVisible = false
      }
    }

    fun navigateToPageIndex(pageIndex: Int) {
      scope.launch {
        val targetPageIndex = pageIndex.coerceIn(0, lastPageIndex)
        if (targetPageIndex != settledPageIndex) {
          onEditorInputRequest()
        }
        activeHardStop = null
        hardStopVisualOffset.snapTo(0f)
        val targetPosition =
          pageMetrics.positionForPageEntry(
            pageIndex = targetPageIndex,
            fromPageIndex = currentPageIndex,
          )
        scrollPosition.animateTo(targetPosition)
        settledPageIndex = targetPageIndex
      }
    }

    fun navigateToPage(page: EditorToolbarPageKey) {
      val pageIndex = pages.indexOfFirst { it.key == page }
      if (pageIndex >= 0) {
        navigateToPageIndex(pageIndex)
      }
    }

    suspend fun settlePages(velocity: Float = 0f) {
      val snapPosition = pageMetrics.snapPosition(scrollPosition.value, velocity, activeHardStop)
      val snapPage = pageMetrics.pageIndexForPosition(snapPosition)
      if (snapPage != settledPageIndex) {
        onEditorInputRequest()
      }
      scrollPosition.animateTo(snapPosition)
      settledPageIndex = snapPage
      activeHardStop = null
      if (hardStopVisualOffset.value != 0f) {
        hardStopVisualOffset.animateTo(0f, ToolbarHardStopOverscrollSpring)
      }
    }

    val flingBehavior =
      remember(scrollPosition, pageMetrics, activeHardStop, onEditorInputRequest) {
        object : FlingBehavior {
          override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
            settlePages(initialVelocity)
            return 0f
          }
        }
      }

    val indicatorAlpha by
      animateFloatAsState(
        targetValue = if (indicatorVisible || indicatorInteracting) 1f else 0f,
        animationSpec = tween(ToolbarIndicatorFadeMillis),
        label = "editor-toolbar-indicator-alpha",
      )

    if (pageCount > 1) {
      EditorToolbarIndicatorPill(
        pages = pages,
        pageProgress = indicatorProgress,
        animateBackground = indicatorInteracting && !indicatorDragging,
        currentPageIndex = currentPageIndex,
        modifier =
          Modifier.align(Alignment.TopCenter)
            .alpha(indicatorAlpha)
            .then(
              if (indicatorAlpha > 0.01f) {
                Modifier.toolbarIndicatorGestures(
                    pageCount = pageCount,
                    currentPageIndex = currentPageIndex,
                    onIndicatorProgress = { progress -> indicatorDragProgress = progress },
                    onIndicatorDraggingChange = { dragging -> indicatorDragging = dragging },
                    onPageSelected = { page -> navigateToPageIndex(page) },
                    onInteractionActiveChange = { active ->
                      indicatorInteracting = active
                      if (!active) {
                        indicatorDragProgress = null
                        indicatorDragging = false
                      }
                      indicatorPulse++
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

      Box(
        modifier =
          Modifier.align(Alignment.BottomCenter)
            .fillMaxWidth()
            .height(ToolbarHeight)
            .shadow(AppTheme.shadows.sm, ToolbarCapsuleShape)
            .pressScale(ToolbarCapsulePressedScale)
            .clip(ToolbarCapsuleShape)
            .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarCapsuleShape)
      ) {
        EditorToolbarSurfaceBackground(shape = ToolbarCapsuleShape)

        Box(
          modifier =
            Modifier.matchParentSize()
              .clipToBounds()
              .emitPressInteractions(toolbarInteractionSource)
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
              Modifier.fillMaxSize().graphicsLayer { translationX = hardStopVisualOffset.value }
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
                    val pageOffset = pageMetrics.pageOffsetFor(index, scrollPosition.value)
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
            icon = if (activeBottomPanel != null) Lucide.CircleX else Lucide.KeyboardOff,
            contentDescription =
              if (activeBottomPanel != null) "하단 패널 닫기"
              else if (editorFocused) "에디터 포커스 해제" else "키보드 닫기",
            onClick = onKeyboardDismissRequest,
            shape = ToolbarFixedActionShape,
            fixedActionSurface = true,
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

private data class ToolbarHardStop(val position: Float, val blockedDirection: Int)

private data class ToolbarScrollResult(
  val position: Float,
  val hardStop: ToolbarHardStop?,
  val rejectedDelta: Float = 0f,
)

private data class ToolbarPageMetrics(
  private val pageDistance: Float,
  private val scrollRanges: List<Int>,
) {
  private val lastPageIndex = scrollRanges.lastIndex
  private val pageStarts: List<Float>
  private val snapPositions: List<Float>
  val maxPosition: Float

  init {
    var position = 0f
    pageStarts = scrollRanges.mapIndexed { index, scrollRange ->
      val pageStart = position
      if (index < lastPageIndex) {
        position += scrollRange.coerceAtLeast(0).toFloat() + pageDistance
      }
      pageStart
    }
    maxPosition =
      if (scrollRanges.isEmpty()) {
        0f
      } else {
        pageStarts.last() + scrollRanges.last().coerceAtLeast(0).toFloat()
      }
    snapPositions = buildSnapPositions()
  }

  fun pageOffsetFor(pageIndex: Int, position: Float): Float {
    val pageStart = pageStarts.getOrNull(pageIndex) ?: return 0f
    return pageStart - position + internalScrollFor(pageIndex, position)
  }

  fun internalScrollFor(pageIndex: Int, position: Float): Float {
    val pageStart = pageStarts.getOrNull(pageIndex) ?: return 0f
    val scrollRange = scrollRanges.getOrNull(pageIndex)?.coerceAtLeast(0)?.toFloat() ?: 0f
    return (position - pageStart).coerceIn(0f, scrollRange)
  }

  fun progressFor(position: Float): Float {
    if (scrollRanges.isEmpty() || pageDistance <= 0f) {
      return 0f
    }

    val boundedPosition = position.coerceIn(0f, maxPosition)
    for (index in 0 until lastPageIndex) {
      val pageStart = pageStarts[index]
      val scrollEnd = pageStart + scrollRanges[index].coerceAtLeast(0)
      val nextPageStart = pageStarts[index + 1]

      if (boundedPosition <= scrollEnd && boundedPosition >= pageStart) {
        return index.toFloat()
      }
      if (boundedPosition <= nextPageStart) {
        val transitionProgress = ((boundedPosition - scrollEnd) / pageDistance).coerceIn(0f, 1f)
        return index + transitionProgress
      }
    }

    return lastPageIndex.toFloat()
  }

  fun pageIndexForPosition(position: Float): Int =
    progressFor(position).roundToInt().coerceIn(0, lastPageIndex.coerceAtLeast(0))

  fun positionForPageEntry(pageIndex: Int, fromPageIndex: Int? = null): Float {
    val coercedPageIndex = pageIndex.coerceIn(0, lastPageIndex.coerceAtLeast(0))
    val pageStart = pageStarts.getOrNull(coercedPageIndex) ?: 0f
    val scrollRange = scrollRanges.getOrNull(coercedPageIndex)?.coerceAtLeast(0) ?: 0
    return if (fromPageIndex != null && fromPageIndex > coercedPageIndex) {
      pageStart + scrollRange
    } else {
      pageStart
    }
  }

  fun applyHardStop(
    currentPosition: Float,
    proposedPosition: Float,
    hardStop: ToolbarHardStop?,
  ): ToolbarScrollResult {
    val boundedCurrent = currentPosition.coerceIn(0f, maxPosition)
    val boundedProposed = proposedPosition.coerceIn(0f, maxPosition)
    val direction = (boundedProposed - boundedCurrent).directionSign()
    if (direction == 0) {
      return ToolbarScrollResult(position = boundedProposed, hardStop = hardStop)
    }

    val nextHardStop =
      if (hardStop != null && boundedCurrent.isNear(hardStop.position)) {
        if (direction == hardStop.blockedDirection) {
          return ToolbarScrollResult(
            position = hardStop.position,
            hardStop = hardStop,
            rejectedDelta = boundedProposed - hardStop.position,
          )
        }
        null
      } else {
        null
      }

    pageStarts.forEachIndexed { index, pageStart ->
      val scrollRange = scrollRanges[index].coerceAtLeast(0).toFloat()
      if (scrollRange <= 0f) {
        return@forEachIndexed
      }

      val scrollEnd = pageStart + scrollRange
      if (direction > 0) {
        if (boundedCurrent < scrollEnd && boundedProposed > scrollEnd) {
          val stop = ToolbarHardStop(position = scrollEnd, blockedDirection = direction)
          return ToolbarScrollResult(
            position = scrollEnd,
            hardStop = stop,
            rejectedDelta = boundedProposed - scrollEnd,
          )
        }
      } else {
        if (boundedCurrent > pageStart && boundedProposed < pageStart) {
          val stop = ToolbarHardStop(position = pageStart, blockedDirection = direction)
          return ToolbarScrollResult(
            position = pageStart,
            hardStop = stop,
            rejectedDelta = boundedProposed - pageStart,
          )
        }
      }
    }

    return ToolbarScrollResult(position = boundedProposed, hardStop = nextHardStop)
  }

  fun snapPosition(position: Float, velocity: Float, hardStop: ToolbarHardStop?): Float {
    if (snapPositions.isEmpty()) {
      return 0f
    }

    val boundedPosition = position.coerceIn(0f, maxPosition)
    if (hardStop != null && boundedPosition.isNear(hardStop.position)) {
      return hardStop.position
    }

    return when {
      isInsideInternalScrollRange(boundedPosition) -> boundedPosition
      velocity <= -ToolbarSwipeVelocityThreshold ->
        snapPositions.firstOrNull { it > boundedPosition + ToolbarSnapPositionEpsilon }
          ?: maxPosition
      velocity >= ToolbarSwipeVelocityThreshold ->
        snapPositions.lastOrNull { it < boundedPosition - ToolbarSnapPositionEpsilon } ?: 0f
      else -> snapPositions.minByOrNull { abs(it - boundedPosition) } ?: 0f
    }
  }

  private fun isInsideInternalScrollRange(position: Float): Boolean {
    pageStarts.forEachIndexed { index, pageStart ->
      val scrollRange = scrollRanges[index].coerceAtLeast(0)
      if (scrollRange > 0 && position > pageStart && position < pageStart + scrollRange) {
        return true
      }
    }
    return false
  }

  private fun buildSnapPositions(): List<Float> {
    val positions = mutableListOf<Float>()

    fun addPosition(position: Float) {
      if (
        positions.lastOrNull()?.let { abs(it - position) <= ToolbarSnapPositionEpsilon } != true
      ) {
        positions += position
      }
    }

    pageStarts.forEachIndexed { index, pageStart ->
      addPosition(pageStart)
      val scrollRange = scrollRanges[index].coerceAtLeast(0)
      if (scrollRange > 0) {
        addPosition(pageStart + scrollRange)
      }
    }
    return positions
  }
}

private fun Float.directionSign(): Int =
  when {
    this > ToolbarSnapPositionEpsilon -> 1
    this < -ToolbarSnapPositionEpsilon -> -1
    else -> 0
  }

private fun Float.isNear(other: Float): Boolean = abs(this - other) <= ToolbarSnapPositionEpsilon

private val ToolbarHardStopOverscrollSpring =
  spring<Float>(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = Spring.StiffnessMedium)

private const val ToolbarSnapPositionEpsilon = 0.5f
