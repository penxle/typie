package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.clampDocumentZoom
import co.typie.editor.computePaginatedZoomBounds
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.EditorPinchSample
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveZoomAnchorDisplayPosition
import kotlin.math.exp

private const val PointerSignalZoomDivisor = 240f

internal data class EditorViewportZoomSemanticConfig(
  val layoutSpec: EditorDocumentLayoutSpec.Paginated,
  val zoomController: EditorZoomController,
  val viewportState: EditorViewportState,
  val uiState: EditorUiState,
  val pageSizes: List<PageSize>,
  val viewportWidth: Float,
  val density: Float,
  val onZoomSnap: () -> Unit,
)

internal class EditorViewportZoomSemantic {
  private var config: EditorViewportZoomSemanticConfig? = null
  private var transformActive = false
  private var pinchSession: PinchSession? = null
  private var pointerSignalZoomActive = false
  private var pointerSignalRawZoom: Float? = null

  fun configure(config: EditorViewportZoomSemanticConfig?) {
    if (config?.isUsable != true && (transformActive || hasActiveZoom)) {
      end()
    }
    this.config = config
  }

  fun beginPinch(sample: EditorPinchSample): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!sample.isUsable) {
      return false
    }
    val nextAnchor = currentConfig.resolveAnchor(sample.focalInRootPx) ?: return false
    val startZoom = currentConfig.zoomController.displayZoom
    val startAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = nextAnchor, displayZoom = startZoom)
        ?: return false

    beginTransform(currentConfig)
    pinchSession =
      PinchSession(
        anchor = nextAnchor,
        startSample = sample,
        startScrollOffset = currentConfig.viewportState.scrollOffset,
        startDisplayZoom = startZoom,
        startAnchorDisplayPosition = startAnchorDisplayPosition,
      )
    return true
  }

  fun updatePinch(sample: EditorPinchSample): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    val session = pinchSession ?: return false
    if (!sample.isUsable) {
      return false
    }

    val previousZoom = currentConfig.zoomController.displayZoom
    val nextZoom =
      setZoom(
        config = currentConfig,
        zoom = session.startDisplayZoom * (sample.distancePx / session.startSample.distancePx),
      )
    val nextAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = session.anchor, displayZoom = nextZoom)
        ?: return false
    val focalDelta =
      (sample.focalInRootPx - session.startSample.focalInRootPx) / currentConfig.density
    currentConfig.viewportState.scrollToTransformTarget(
      offset =
        session.startScrollOffset +
          (nextAnchorDisplayPosition - session.startAnchorDisplayPosition) - focalDelta,
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    return true
  }

  fun beginPointerSignal(): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    beginTransform(currentConfig)
    pointerSignalZoomActive = true
    pointerSignalRawZoom = null
    return true
  }

  fun updatePointerSignal(focalInEditorPx: Offset, normalizedDelta: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!normalizedDelta.isFinite() || normalizedDelta == 0f || !pointerSignalZoomActive) {
      return false
    }

    val viewportState = currentConfig.viewportState
    val effectiveScrollTarget = viewportState.effectiveTransformScrollTarget
    val unappliedScrollDelta = effectiveScrollTarget - viewportState.scrollOffset
    val focalInEditor = currentConfig.toEditorDp(focalInEditorPx) + unappliedScrollDelta
    val previousZoom = currentConfig.zoomController.displayZoom
    val anchor =
      currentConfig
        .resolveViewportTransform(displayZoom = previousZoom)
        .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y) ?: return false
    val previousAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = anchor, displayZoom = previousZoom)
        ?: return false
    val baseZoom = pointerSignalRawZoom ?: previousZoom
    val nextRawZoom =
      clampDocumentZoom(
        zoom = baseZoom * exp((-normalizedDelta / PointerSignalZoomDivisor).toDouble()).toFloat(),
        bounds = computePaginatedZoomBounds(currentConfig.layoutSpec.pageWidth),
      )
    pointerSignalRawZoom = nextRawZoom

    val nextZoom = setZoom(config = currentConfig, zoom = nextRawZoom)
    val nextAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = anchor, displayZoom = nextZoom)
        ?: return false
    viewportState.scrollToTransformTarget(
      offset = effectiveScrollTarget + (nextAnchorDisplayPosition - previousAnchorDisplayPosition),
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    return true
  }

  fun end() {
    if (hasActiveZoom) {
      config?.zoomController?.commitRenderZoom()
    }
    pinchSession = null
    pointerSignalZoomActive = false
    pointerSignalRawZoom = null
    if (transformActive) {
      config?.viewportState?.endTransform()
      transformActive = false
    }
  }

  fun reset() {
    end()
    config = null
  }

  private val hasActiveZoom: Boolean
    get() = pinchSession != null || pointerSignalZoomActive

  private fun beginTransform(config: EditorViewportZoomSemanticConfig) {
    if (!transformActive) {
      config.viewportState.beginTransform()
      transformActive = true
    }
  }

  private fun setZoom(config: EditorViewportZoomSemanticConfig, zoom: Float): Float {
    val previousZoom = config.zoomController.displayZoom
    val previousSnap = config.zoomController.resolveSnapKey(previousZoom)
    val changed =
      config.zoomController.setDisplayZoom(
        zoom = zoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
    val nextZoom = config.zoomController.displayZoom
    if (changed) {
      maybeSendZoomSnapHaptic(
        previousSnap = previousSnap,
        nextSnap = config.zoomController.resolveSnapKey(nextZoom),
        haptic = config.onZoomSnap,
      )
    }
    return nextZoom
  }
}

private data class PinchSession(
  val anchor: EditorViewportAnchor,
  val startSample: EditorPinchSample,
  val startScrollOffset: Offset,
  val startDisplayZoom: Float,
  val startAnchorDisplayPosition: Offset,
)

private val EditorViewportZoomSemanticConfig.isUsable: Boolean
  get() = density > 0f && viewportWidth > 0f && layoutSpec.pageWidth > 0f && pageSizes.isNotEmpty()

private val EditorPinchSample.isUsable: Boolean
  get() =
    focalInRootPx.x.isFinite() &&
      focalInRootPx.y.isFinite() &&
      distancePx.isFinite() &&
      distancePx > 0f

private fun EditorViewportZoomSemanticConfig.resolveAnchor(
  focalInRootPx: Offset
): EditorViewportAnchor? {
  val editorRect = uiState.editorRectInRoot() ?: return null
  val focal = toEditorDp(focalInRootPx - editorRect.topLeft)
  return resolveViewportTransform(displayZoom = null)
    .resolveAnchor(focalX = focal.x, focalY = focal.y)
}

private fun EditorViewportZoomSemanticConfig.resolveAnchorDisplayPosition(
  anchor: EditorViewportAnchor,
  displayZoom: Float,
): Offset? =
  resolveZoomAnchorDisplayPosition(
    layoutSpec = layoutSpec,
    anchor = anchor,
    displayZoom = displayZoom,
    viewportWidth = viewportWidth,
    pageSizes = pageSizes,
    density = density,
  )

private fun EditorViewportZoomSemanticConfig.resolveViewportTransform(displayZoom: Float?) =
  uiState.resolveViewportTransform(pageSizes = pageSizes).let { transform ->
    if (displayZoom == null) {
      transform
    } else {
      transform.copy(displayZoom = displayZoom)
    }
  }

private fun EditorViewportZoomSemanticConfig.toEditorDp(focalPx: Offset): Offset =
  Offset(x = focalPx.x / density, y = focalPx.y / density)

private fun maybeSendZoomSnapHaptic(
  previousSnap: EditorZoomSnapKey?,
  nextSnap: EditorZoomSnapKey?,
  haptic: () -> Unit,
) {
  if (nextSnap == null || nextSnap == previousSnap) {
    return
  }

  haptic()
}
