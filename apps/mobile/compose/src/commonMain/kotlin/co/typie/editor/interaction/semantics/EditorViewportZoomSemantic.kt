package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.clampDocumentZoom
import co.typie.editor.computePaginatedZoomBounds
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.syncViewportToZoomAnchor
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
  private var pinchAnchor: EditorViewportAnchor? = null
  private var pinchStartDistancePx = 0f
  private var pinchStartZoom = 1f
  private var pointerSignalZoomActive = false
  private var pointerSignalRawZoom: Float? = null

  fun configure(config: EditorViewportZoomSemanticConfig?) {
    if (config?.isUsable != true && (transformActive || hasActiveZoom)) {
      end()
    }
    this.config = config
  }

  fun beginPinch(focalPx: Offset, distancePx: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!distancePx.isFinite() || distancePx <= 0f) {
      return false
    }
    val nextAnchor = currentConfig.resolveAnchor(focalPx) ?: return false

    beginTransform(currentConfig)
    pinchAnchor = nextAnchor
    pinchStartDistancePx = distancePx
    pinchStartZoom = currentConfig.zoomController.displayZoom
    return true
  }

  fun updatePinch(focalPx: Offset, distancePx: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    val currentAnchor = pinchAnchor ?: return false
    if (!distancePx.isFinite() || distancePx <= 0f || pinchStartDistancePx <= 0f) {
      return false
    }

    val focal = currentConfig.toEditorDp(focalPx)
    val nextZoom =
      setZoom(config = currentConfig, zoom = pinchStartZoom * (distancePx / pinchStartDistancePx))
    syncViewportToZoomAnchor(
      viewportState = currentConfig.viewportState,
      layoutSpec = currentConfig.layoutSpec,
      pageSizes = currentConfig.pageSizes,
      anchor = currentAnchor,
      focalX = focal.x,
      focalY = focal.y,
      displayZoom = nextZoom,
      density = currentConfig.density,
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

  fun updatePointerSignal(focalPx: Offset, normalizedDelta: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!normalizedDelta.isFinite() || normalizedDelta == 0f || !pointerSignalZoomActive) {
      return false
    }

    val focal = currentConfig.toEditorDp(focalPx)
    val previousZoom = currentConfig.zoomController.displayZoom
    val anchor =
      currentConfig
        .resolveViewportTransform(displayZoom = previousZoom)
        .resolveAnchor(focalX = focal.x, focalY = focal.y) ?: return false
    val baseZoom = pointerSignalRawZoom ?: previousZoom
    val nextRawZoom =
      clampDocumentZoom(
        zoom = baseZoom * exp((-normalizedDelta / PointerSignalZoomDivisor).toDouble()).toFloat(),
        bounds = computePaginatedZoomBounds(currentConfig.layoutSpec.pageWidth),
      )
    pointerSignalRawZoom = nextRawZoom

    val nextZoom = setZoom(config = currentConfig, zoom = nextRawZoom)
    syncViewportToZoomAnchor(
      viewportState = currentConfig.viewportState,
      layoutSpec = currentConfig.layoutSpec,
      pageSizes = currentConfig.pageSizes,
      anchor = anchor,
      focalX = focal.x,
      focalY = focal.y,
      displayZoom = nextZoom,
      density = currentConfig.density,
    )
    return true
  }

  fun end() {
    if (hasActiveZoom) {
      config?.zoomController?.commitRenderZoom()
    }
    pinchAnchor = null
    pinchStartDistancePx = 0f
    pinchStartZoom = 1f
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
    get() = pinchAnchor != null || pointerSignalZoomActive

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

private val EditorViewportZoomSemanticConfig.isUsable: Boolean
  get() = density > 0f && viewportWidth > 0f && layoutSpec.pageWidth > 0f && pageSizes.isNotEmpty()

private fun EditorViewportZoomSemanticConfig.resolveAnchor(focalPx: Offset): EditorViewportAnchor? {
  val focal = toEditorDp(focalPx)
  return resolveViewportTransform(displayZoom = null)
    .resolveAnchor(focalX = focal.x, focalY = focal.y)
}

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
