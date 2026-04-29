package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.roundToInt

@Composable
internal fun rememberToolbarPagerState(): ToolbarPagerState = remember { ToolbarPagerState() }

@Stable
internal class ToolbarPagerState {
  var scrollPosition by mutableFloatStateOf(0f)
  val scrollPositionAnimation = Animatable(0f)
  val hardStopVisualOffset = Animatable(0f)
  var indicatorVisible by mutableStateOf(false)
  var indicatorInteracting by mutableStateOf(false)
  var indicatorDragging by mutableStateOf(false)
  var indicatorPulse by mutableIntStateOf(0)
  var indicatorDragProgress by mutableStateOf<Float?>(null)
  var indicatorPageTransitioning by mutableStateOf(false)
  var settledPageIndex by mutableIntStateOf(0)
  var activeHardStop by mutableStateOf<ToolbarHardStop?>(null)
  var scrollGestureStartPosition by mutableStateOf<Float?>(null)
  var pointerScrollGestureActive by mutableStateOf(false)
  var decayFlingInProgress by mutableStateOf(false)
  var decayHardStopBounceStarted by mutableStateOf(false)
}

internal data class ToolbarHardStop(val position: Float, val blockedDirection: Int)

internal data class ToolbarScrollResult(
  val position: Float,
  val hardStop: ToolbarHardStop?,
  val rejectedDelta: Float = 0f,
)

internal data class ToolbarPagerMetrics(
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

  fun decaysFlingWithinInternalScroll(position: Float, velocity: Float): Boolean {
    if (velocity.directionSign() == 0) {
      return false
    }

    val boundedPosition = position.coerceIn(0f, maxPosition)
    pageStarts.forEachIndexed { index, pageStart ->
      val scrollRange = scrollRanges[index].coerceAtLeast(0)
      if (scrollRange <= 0) {
        return@forEachIndexed
      }

      val scrollEnd = pageStart + scrollRange
      if (boundedPosition > pageStart && boundedPosition < scrollEnd) {
        return true
      }
      if (boundedPosition.isNear(pageStart) && velocity < 0f) {
        return true
      }
      if (boundedPosition.isNear(scrollEnd) && velocity > 0f) {
        return true
      }
    }
    return false
  }

  fun isPageTransitionPosition(position: Float): Boolean {
    if (scrollRanges.isEmpty() || pageDistance <= 0f) {
      return false
    }

    val boundedPosition = position.coerceIn(0f, maxPosition)
    for (index in 0 until lastPageIndex) {
      val pageStart = pageStarts[index]
      val scrollEnd = pageStart + scrollRanges[index].coerceAtLeast(0)
      val nextPageStart = pageStarts[index + 1]
      if (
        boundedPosition > scrollEnd + ToolbarSnapPositionEpsilon &&
          boundedPosition < nextPageStart - ToolbarSnapPositionEpsilon
      ) {
        return true
      }
    }
    return false
  }

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
    gestureStartPosition: Float?,
    activationEpsilon: Float,
  ): ToolbarScrollResult {
    val boundedCurrent = currentPosition.coerceIn(0f, maxPosition)
    val boundedProposed = proposedPosition.coerceIn(0f, maxPosition)
    val boundedGestureStart = gestureStartPosition?.coerceIn(0f, maxPosition)
    val boundedActivationEpsilon = activationEpsilon.coerceAtLeast(0f)
    val direction = (boundedProposed - boundedCurrent).directionSign()
    if (direction == 0) {
      return ToolbarScrollResult(position = boundedProposed, hardStop = hardStop)
    }

    val nextHardStop =
      if (hardStop != null && boundedCurrent.isNear(hardStop.position)) {
        val startedNearHardStop =
          boundedGestureStart != null &&
            abs(boundedGestureStart - hardStop.position) <= boundedActivationEpsilon
        if (direction == hardStop.blockedDirection && !startedNearHardStop) {
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
      val edgeActivationEpsilon = min(boundedActivationEpsilon, scrollRange / 2f)
      if (direction > 0) {
        val startedNearScrollEnd =
          boundedGestureStart != null &&
            abs(boundedGestureStart - scrollEnd) <= edgeActivationEpsilon
        if (boundedCurrent <= scrollEnd && boundedProposed > scrollEnd && !startedNearScrollEnd) {
          val stop = ToolbarHardStop(position = scrollEnd, blockedDirection = direction)
          return ToolbarScrollResult(
            position = scrollEnd,
            hardStop = stop,
            rejectedDelta = boundedProposed - scrollEnd,
          )
        }
      } else {
        val startedNearScrollStart =
          boundedGestureStart != null &&
            abs(boundedGestureStart - pageStart) <= edgeActivationEpsilon
        if (boundedCurrent >= pageStart && boundedProposed < pageStart && !startedNearScrollStart) {
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

internal val ToolbarHardStopOverscrollSpring =
  spring<Float>(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = Spring.StiffnessMedium)

internal const val ToolbarSnapPositionEpsilon = 0.5f
