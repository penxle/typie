package co.typie.editor

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import co.typie.editor.body.EditorDocumentLayoutSpec
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val MinDocumentDisplayWidth = 100f
private const val MaxDocumentZoom = 2f
private const val FitWidthZoomSnapThreshold = 0.02f
private const val UnitZoomSnapThreshold = 0.02f
private const val ZoomEpsilon = 0.0001f
private const val RenderZoomDebounceMs = 120L

@Stable
internal class EditorZoomController(
  private val scope: CoroutineScope? = null,
  private val renderZoomDebounceMs: Long = RenderZoomDebounceMs,
) {
  var displayZoom by mutableFloatStateOf(1f)
    private set

  var renderZoom by mutableFloatStateOf(1f)
    private set

  val isPaginated: Boolean
    get() = currentLayoutSpec is EditorDocumentLayoutSpec.Paginated

  private var initializedPaginatedPageWidth: Float = Float.NaN
  private var currentLayoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(0f)
  private var currentViewportWidth: Float = 0f
  private var renderZoomJob: Job? = null

  fun syncLayout(layoutSpec: EditorDocumentLayoutSpec, viewportWidth: Float) {
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth

    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> {
        initializedPaginatedPageWidth = Float.NaN
        setZoomInternal(
          zoom = 1f,
          layoutSpec = layoutSpec,
          viewportWidth = viewportWidth,
          commitRender = true,
        )
      }

      is EditorDocumentLayoutSpec.Paginated -> {
        if (layoutSpec.pageWidth <= 0f || viewportWidth <= 0f) {
          return
        }

        val shouldApplyInitialZoom =
          initializedPaginatedPageWidth.isNaN() ||
            abs(initializedPaginatedPageWidth - layoutSpec.pageWidth) >= ZoomEpsilon
        if (shouldApplyInitialZoom) {
          initializedPaginatedPageWidth = layoutSpec.pageWidth
          setZoomInternal(
            zoom =
              computeInitialPaginatedZoom(
                pageWidth = layoutSpec.pageWidth,
                viewportWidth = viewportWidth,
              ),
            layoutSpec = layoutSpec,
            viewportWidth = viewportWidth,
            commitRender = true,
          )
          return
        }

        setZoomInternal(
          zoom = displayZoom,
          layoutSpec = layoutSpec,
          viewportWidth = viewportWidth,
          commitRender = true,
        )
      }
    }
  }

  fun setDisplayZoom(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
  ): Boolean {
    return setZoomInternal(
      zoom = zoom,
      layoutSpec = layoutSpec,
      viewportWidth = viewportWidth,
      commitRender = false,
    )
  }

  fun commitRenderZoom() {
    renderZoomJob?.cancel()
    renderZoomJob = null
    syncRenderZoomNow()
  }

  fun resolveSnapKey(zoom: Float = displayZoom): EditorZoomSnapKey? {
    val layout = currentLayoutSpec as? EditorDocumentLayoutSpec.Paginated ?: return null
    val viewportWidth = resolveViewportWidthFallback(layout.pageWidth)
    val fitWidthZoom =
      computePaginatedFitWidthZoom(pageWidth = layout.pageWidth, viewportWidth = viewportWidth)
    val unitZoom =
      clampDocumentZoom(zoom = 1f, bounds = computePaginatedZoomBounds(layout.pageWidth))

    return when {
      zoomEquals(zoom, fitWidthZoom) -> EditorZoomSnapKey.FitWidth
      zoomEquals(zoom, unitZoom) -> EditorZoomSnapKey.Unit
      else -> null
    }
  }

  private fun resolveDisplayZoom(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
  ): Float =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> 1f
      is EditorDocumentLayoutSpec.Paginated ->
        clampPaginatedZoom(
          zoom = zoom,
          pageWidth = layoutSpec.pageWidth,
          viewportWidth = viewportWidth,
        )
    }

  private fun setZoomInternal(
    zoom: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    viewportWidth: Float,
    commitRender: Boolean,
  ): Boolean {
    currentLayoutSpec = layoutSpec
    currentViewportWidth = viewportWidth

    val resolvedZoom =
      resolveDisplayZoom(zoom = zoom, layoutSpec = layoutSpec, viewportWidth = viewportWidth)
    val changed = zoomDiffers(displayZoom, resolvedZoom)
    if (changed) {
      displayZoom = resolvedZoom
    }

    renderZoomJob?.cancel()
    renderZoomJob = null

    if (commitRender) {
      syncRenderZoomNow()
      return changed
    }

    renderZoomJob = scope?.launch {
      delay(renderZoomDebounceMs)
      syncRenderZoomNow()
    }
    return changed
  }

  private fun syncRenderZoomNow() {
    val nextRenderZoom =
      when (currentLayoutSpec) {
        is EditorDocumentLayoutSpec.Continuous -> 1f
        is EditorDocumentLayoutSpec.Paginated -> renderZoomForDisplay(displayZoom)
      }
    if (!zoomEquals(renderZoom, nextRenderZoom)) {
      renderZoom = nextRenderZoom
    }
  }

  private fun resolveViewportWidthFallback(pageWidth: Float): Float {
    return if (currentViewportWidth.isFinite() && currentViewportWidth > 0f) {
      currentViewportWidth
    } else if (pageWidth.isFinite() && pageWidth > 0f) {
      pageWidth
    } else {
      1f
    }
  }
}

internal enum class EditorZoomSnapKey {
  FitWidth,
  Unit,
}

internal fun computePaginatedZoomBounds(pageWidth: Float): ClosedFloatingPointRange<Float> {
  val safePageWidth = if (pageWidth.isFinite() && pageWidth > 0f) pageWidth else 1f
  val minZoom = (MinDocumentDisplayWidth / safePageWidth).coerceAtLeast(0.01f)
  val maxZoom = MaxDocumentZoom.coerceAtLeast(minZoom)
  return minZoom..maxZoom
}

internal fun clampDocumentZoom(zoom: Float, bounds: ClosedFloatingPointRange<Float>): Float {
  if (!zoom.isFinite()) {
    return bounds.start
  }

  return zoom.coerceIn(bounds.start, bounds.endInclusive)
}

internal fun computePaginatedFitWidthZoom(pageWidth: Float, viewportWidth: Float): Float {
  val bounds = computePaginatedZoomBounds(pageWidth)
  val safePageWidth = if (pageWidth.isFinite() && pageWidth > 0f) pageWidth else 1f
  val safeViewportWidth =
    if (viewportWidth.isFinite() && viewportWidth > 0f) {
      viewportWidth
    } else {
      safePageWidth
    }
  return (safeViewportWidth / safePageWidth).coerceIn(bounds.start, bounds.endInclusive)
}

internal fun computeInitialPaginatedZoom(pageWidth: Float, viewportWidth: Float): Float =
  computePaginatedFitWidthZoom(pageWidth = pageWidth, viewportWidth = viewportWidth)
    .coerceAtMost(1f)

internal fun clampPaginatedZoom(zoom: Float, pageWidth: Float, viewportWidth: Float): Float {
  val bounds = computePaginatedZoomBounds(pageWidth)
  val clamped = clampDocumentZoom(zoom = zoom, bounds = bounds)
  val fitWidthZoom =
    computePaginatedFitWidthZoom(pageWidth = pageWidth, viewportWidth = viewportWidth)
  val unitZoom = clampDocumentZoom(zoom = 1f, bounds = bounds)

  var snapped: Float? = null
  var bestDistance = Float.POSITIVE_INFINITY

  val fitWidthDistance = abs(clamped - fitWidthZoom)
  if (fitWidthDistance <= FitWidthZoomSnapThreshold) {
    snapped = fitWidthZoom
    bestDistance = fitWidthDistance
  }

  val unitDistance = abs(clamped - unitZoom)
  if (unitDistance <= UnitZoomSnapThreshold && unitDistance < bestDistance) {
    snapped = unitZoom
  }

  return snapped ?: clamped
}

internal fun renderZoomForDisplay(displayZoom: Float): Float {
  if (!displayZoom.isFinite()) {
    return 1f
  }

  return if (displayZoom <= 0f) 0.01f else displayZoom
}

internal fun zoomEquals(a: Float, b: Float): Boolean = abs(a - b) < ZoomEpsilon

internal fun zoomDiffers(a: Float, b: Float): Boolean = !zoomEquals(a, b)

@Composable
internal fun rememberEditorZoomController(key: Any): EditorZoomController {
  val scope = rememberCoroutineScope()
  return remember(key) { EditorZoomController(scope = scope) }
}

internal val LocalEditorZoomController =
  compositionLocalOf<EditorZoomController> { error("No EditorZoomController provided") }
