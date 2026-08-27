package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.documentZoomWidth
import co.typie.editor.clampDocumentZoom
import co.typie.editor.computeDocumentZoomBounds
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.EditorPinchSample
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveZoomAnchorDisplayPosition
import kotlin.math.exp

private const val PointerSignalZoomDivisor = 240f

internal data class EditorViewportZoomSemanticConfig(
  val layoutSpec: EditorDocumentLayoutSpec,
  val zoomController: EditorZoomController,
  val viewportState: EditorViewportState,
  val uiState: EditorUiState,
  val pageSizes: List<PageSize>,
  val viewportWidth: Float,
  val density: Float,
  val onZoomSnap: () -> Unit,
  val onAttachViewportAnchor:
    (anchor: EditorViewportAnchor, displayPosition: Offset, scrollOffset: Offset) -> Unit =
    { _, _, _ ->
    },
)

internal class EditorViewportZoomSemantic {
  private var config: EditorViewportZoomSemanticConfig? = null
  private var transformActive = false
  private var pinchSession: PinchSession? = null
  private var indirectZoomActive = false
  private var indirectRawZoom: Float? = null

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
        pageSizes = currentConfig.pageSizes,
        lastSample = sample,
      )
    return true
  }

  fun updatePinch(sample: EditorPinchSample): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    var session = pinchSession ?: return false
    if (!sample.isUsable) {
      return false
    }
    if (currentConfig.pageSizes !== session.pageSizes) {
      session = currentConfig.rebasePinchSession(session) ?: return false
      pinchSession = session
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
    val targetScrollOffset =
      session.startScrollOffset + (nextAnchorDisplayPosition - session.startAnchorDisplayPosition) -
        focalDelta
    currentConfig.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    currentConfig.onAttachViewportAnchor(
      session.anchor,
      nextAnchorDisplayPosition,
      targetScrollOffset,
    )
    pinchSession = session.copy(lastSample = sample)
    return true
  }

  fun beginIndirect(): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    beginTransform(currentConfig)
    indirectZoomActive = true
    indirectRawZoom = null
    return true
  }

  fun updateIndirectScroll(focalInRootPx: Offset, normalizedDelta: Float): Boolean {
    if (!normalizedDelta.isFinite() || normalizedDelta == 0f) {
      return false
    }
    return updateIndirectScale(
      focalInRootPx = focalInRootPx,
      scaleFactor = exp((-normalizedDelta / PointerSignalZoomDivisor).toDouble()).toFloat(),
    )
  }

  fun updateIndirectScale(focalInRootPx: Offset, scaleFactor: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!indirectZoomActive || !isValidScaleUpdate(focalInRootPx, scaleFactor)) {
      return false
    }
    if (scaleFactor == 1f) {
      return true
    }

    val viewportState = currentConfig.viewportState
    val effectiveScrollTarget = viewportState.effectiveTransformScrollTarget
    val unappliedScrollDelta = effectiveScrollTarget - viewportState.scrollOffset
    val editorRect = currentConfig.uiState.editorRectInRoot() ?: return false
    val focalInEditor =
      currentConfig.toEditorDp(focalInRootPx - editorRect.topLeft) + unappliedScrollDelta
    val previousZoom = currentConfig.zoomController.displayZoom
    val anchor =
      currentConfig
        .resolveViewportTransform(displayZoom = previousZoom)
        .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y) ?: return false
    val previousAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = anchor, displayZoom = previousZoom)
        ?: return false
    val baseZoom = indirectRawZoom ?: previousZoom
    val nextRawZoom =
      clampDocumentZoom(
        zoom = baseZoom * scaleFactor,
        bounds = computeDocumentZoomBounds(currentConfig.layoutSpec),
      )
    indirectRawZoom = nextRawZoom

    val nextZoom = setZoom(config = currentConfig, zoom = nextRawZoom)
    val nextAnchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = anchor, displayZoom = nextZoom)
        ?: return false
    val targetScrollOffset =
      effectiveScrollTarget + (nextAnchorDisplayPosition - previousAnchorDisplayPosition)
    viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    currentConfig.onAttachViewportAnchor(anchor, nextAnchorDisplayPosition, targetScrollOffset)
    return true
  }

  fun end() {
    if (hasActiveZoom) {
      config?.zoomController?.commitRenderZoom()
    }
    pinchSession = null
    indirectZoomActive = false
    indirectRawZoom = null
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
    get() = pinchSession != null || indirectZoomActive

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
  val pageSizes: List<PageSize>,
  val lastSample: EditorPinchSample,
)

private fun EditorViewportZoomSemanticConfig.rebasePinchSession(
  session: PinchSession
): PinchSession? {
  val anchor = resolveAnchor(session.lastSample.focalInRootPx) ?: return null
  val displayZoom = zoomController.displayZoom
  val anchorDisplayPosition =
    resolveAnchorDisplayPosition(anchor = anchor, displayZoom = displayZoom) ?: return null
  return PinchSession(
    anchor = anchor,
    startSample = session.lastSample,
    startScrollOffset = viewportState.scrollOffset,
    startDisplayZoom = displayZoom,
    startAnchorDisplayPosition = anchorDisplayPosition,
    pageSizes = pageSizes,
    lastSample = session.lastSample,
  )
}

private val EditorViewportZoomSemanticConfig.isUsable: Boolean
  get() =
    density > 0f &&
      viewportWidth > 0f &&
      layoutSpec.documentZoomWidth() > 0f &&
      pageSizes.isNotEmpty()

private val EditorPinchSample.isUsable: Boolean
  get() =
    focalInRootPx.x.isFinite() &&
      focalInRootPx.y.isFinite() &&
      distancePx.isFinite() &&
      distancePx > 0f

private fun isValidScaleUpdate(focalInRootPx: Offset, scaleFactor: Float): Boolean {
  if (!focalInRootPx.x.isFinite() || !focalInRootPx.y.isFinite()) {
    return false
  }
  return scaleFactor.isFinite() && scaleFactor > 0f
}

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
