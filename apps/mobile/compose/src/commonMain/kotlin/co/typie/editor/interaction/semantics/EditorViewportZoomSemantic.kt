package co.typie.editor.interaction.semantics

import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomLandmark
import co.typie.editor.EditorZoomMotion
import co.typie.editor.EditorZoomMotionFrame
import co.typie.editor.EditorZoomMotionTuning
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.documentZoomWidth
import co.typie.editor.clampDocumentLayoutZoom
import co.typie.editor.clampDocumentZoom
import co.typie.editor.computeDocumentZoomBounds
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.EditorPinchSample
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveZoomAnchorDisplayPosition
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.ln
import kotlin.time.TimeSource
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

private const val DirectSnapVelocityThreshold = 0.18

private const val PointerSignalZoomDivisor = 240f
private val ZoomTimeOrigin = TimeSource.Monotonic.markNow()

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

internal class EditorViewportZoomSemantic(
  private val coroutineScope: CoroutineScope? = null,
  private val nowMillis: () -> Long = { ZoomTimeOrigin.elapsedNow().inWholeMilliseconds },
) {
  private var config: EditorViewportZoomSemanticConfig? = null
  private var transformActive = false
  private var pinchSession: PinchSession? = null
  private var indirectZoomActive = false
  private var indirectSession: IndirectSession? = null
  private var motionJob: Job? = null
  private var activeMotion: ActiveMotion? = null
  private var lastSnapKey: EditorZoomSnapKey? = null

  fun configure(config: EditorViewportZoomSemanticConfig?) {
    val previous = this.config
    val ownerChanged =
      previous != null &&
        config != null &&
        (previous.zoomController !== config.zoomController ||
          previous.viewportState !== config.viewportState ||
          previous.layoutSpec != config.layoutSpec)
    if ((config?.isUsable != true || ownerChanged) && hasInteraction) {
      abortForConfigurationChange(previous)
    }
    this.config = config
  }

  fun beginPinch(sample: EditorPinchSample): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!sample.isUsable) return false
    stopMotion(commitRender = false)
    val anchor = currentConfig.resolveAnchor(sample.focalInRootPx) ?: return false
    val startZoom = currentConfig.zoomController.displayZoom
    val anchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = anchor, displayZoom = startZoom)
        ?: return false
    beginTransform(currentConfig)
    val focal = currentConfig.toRootDp(sample.focalInRootPx)
    pinchSession =
      PinchSession(
        anchor = anchor,
        startSample = sample,
        startScrollOffset = currentConfig.viewportState.effectiveTransformScrollTarget,
        startRawZoom = startZoom,
        startAnchorDisplayPosition = anchorDisplayPosition,
        startFocal = focal,
        pageSizes = currentConfig.pageSizes,
        lastSample = sample,
        rawZoom = startZoom,
      )
    lastSnapKey = currentConfig.zoomController.resolveSnapKey(startZoom)
    return true
  }

  fun updatePinch(sample: EditorPinchSample): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    var session = pinchSession ?: return false
    if (!sample.isUsable) return false
    if (currentConfig.pageSizes !== session.pageSizes) {
      session = currentConfig.rebasePinchSession(session) ?: return false
      pinchSession = session
    }

    val rawZoom = session.startRawZoom * (sample.distancePx / session.startSample.distancePx)
    val previousZoom = currentConfig.zoomController.displayZoom
    val nextZoom =
      setGestureZoom(
        config = currentConfig,
        rawZoom = rawZoom,
        previousRawZoom = session.rawZoom,
        previousTimestampMillis = session.lastSample.timestampMillis,
        timestampMillis = sample.timestampMillis,
      )
    val anchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = session.anchor, displayZoom = nextZoom)
        ?: return false
    val focal = currentConfig.toRootDp(sample.focalInRootPx)
    val targetScrollOffset =
      session.startScrollOffset + (anchorDisplayPosition - session.startAnchorDisplayPosition) -
        (focal - session.startFocal)
    currentConfig.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    currentConfig.onAttachViewportAnchor(session.anchor, anchorDisplayPosition, targetScrollOffset)
    pinchSession = session.copy(lastSample = sample, rawZoom = rawZoom)
    return true
  }

  fun beginIndirect(): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    stopMotion(commitRender = false)
    beginTransform(currentConfig)
    indirectZoomActive = true
    indirectSession = null
    lastSnapKey = currentConfig.zoomController.resolveSnapKey()
    return true
  }

  fun updateIndirectScroll(focalInRootPx: Offset, normalizedDelta: Float): Boolean {
    if (!normalizedDelta.isFinite() || normalizedDelta == 0f) return false
    return updateIndirectScale(
      focalInRootPx = focalInRootPx,
      scaleFactor = exp((-normalizedDelta / PointerSignalZoomDivisor).toDouble()).toFloat(),
    )
  }

  fun updateIndirectScale(focalInRootPx: Offset, scaleFactor: Float): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!indirectZoomActive || !isValidScaleUpdate(focalInRootPx, scaleFactor)) return false
    if (scaleFactor == 1f) return true
    val timestampMillis = nowMillis()
    var session =
      indirectSession
        ?: currentConfig.createIndirectSession(focalInRootPx, timestampMillis)
        ?: return false
    if (currentConfig.pageSizes !== session.pageSizes) {
      session = currentConfig.createIndirectSession(focalInRootPx, timestampMillis) ?: return false
    }

    val rawZoom = session.rawZoom * scaleFactor
    val previousZoom = currentConfig.zoomController.displayZoom
    val nextZoom =
      setGestureZoom(
        config = currentConfig,
        rawZoom = rawZoom,
        previousRawZoom = session.rawZoom,
        previousTimestampMillis = session.lastTimestampMillis,
        timestampMillis = timestampMillis,
      )
    val anchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor = session.anchor, displayZoom = nextZoom)
        ?: return false
    val focal = currentConfig.toRootDp(focalInRootPx)
    val targetScrollOffset =
      session.startScrollOffset + (anchorDisplayPosition - session.startAnchorDisplayPosition) -
        (focal - session.startFocal)
    currentConfig.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = previousZoom != nextZoom,
    )
    currentConfig.onAttachViewportAnchor(session.anchor, anchorDisplayPosition, targetScrollOffset)
    indirectSession =
      session.copy(
        lastTimestampMillis = maxOf(session.lastTimestampMillis, timestampMillis),
        rawZoom = rawZoom,
      )
    return true
  }

  fun release() {
    val currentConfig = config?.takeIf { it.isUsable }
    val seed = currentConfig?.createMotionSeed()
    clearDirectSessions()
    if (currentConfig == null || seed == null) {
      settleAndFinishInteraction(currentConfig)
      return
    }
    finishOrRecover(config = currentConfig, seed = seed)
  }

  fun setZoomAtViewportCenter(targetZoom: Float, snapToLandmarks: Boolean = true): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (!targetZoom.isFinite() || targetZoom <= 0f) return false
    val viewportSize = currentConfig.viewportState.viewportSize
    if (viewportSize.width <= 0f || viewportSize.height <= 0f) return false
    val previousZoom = currentConfig.zoomController.displayZoom
    val startScrollOffset = currentConfig.viewportState.effectiveTransformScrollTarget
    val viewportCenter =
      startScrollOffset + Offset(x = viewportSize.width / 2f, y = viewportSize.height / 2f)
    val anchor =
      currentConfig
        .resolveViewportTransform(displayZoom = previousZoom)
        .resolveAnchor(focalX = viewportCenter.x, focalY = viewportCenter.y) ?: return false
    val previousAnchorPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor, previousZoom) ?: return false
    val resolvedZoom =
      if (snapToLandmarks) {
        clampDocumentLayoutZoom(
          zoom = targetZoom,
          layoutSpec = currentConfig.layoutSpec,
          viewportWidth = currentConfig.viewportWidth,
        )
      } else {
        clampDocumentZoom(targetZoom, computeDocumentZoomBounds(currentConfig.layoutSpec))
      }
    if (previousZoom == resolvedZoom) return false

    clearDirectSessions()
    stopMotion(commitRender = false)
    beginTransform(currentConfig)
    val changed =
      currentConfig.zoomController.setDisplayZoom(
        zoom = resolvedZoom,
        layoutSpec = currentConfig.layoutSpec,
        viewportWidth = currentConfig.viewportWidth,
        snapToLandmarks = snapToLandmarks,
      )
    val anchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(anchor, currentConfig.zoomController.displayZoom)
    if (!changed || anchorDisplayPosition == null) {
      finishInteraction(currentConfig)
      return false
    }
    val targetScrollOffset = startScrollOffset + (anchorDisplayPosition - previousAnchorPosition)
    currentConfig.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = true,
    )
    currentConfig.onAttachViewportAnchor(anchor, anchorDisplayPosition, targetScrollOffset)
    finishInteraction(currentConfig)
    return true
  }

  fun zoomInAtViewportCenter(): Boolean {
    val currentConfig = config ?: return false
    val targetZoom = currentConfig.zoomController.resolveZoomInTarget() ?: return false
    val changed = setZoomAtViewportCenter(targetZoom, snapToLandmarks = false)
    if (changed) currentConfig.onZoomSnap()
    return changed
  }

  fun zoomOutAtViewportCenter(): Boolean {
    val currentConfig = config ?: return false
    val targetZoom = currentConfig.zoomController.resolveZoomOutTarget() ?: return false
    val changed = setZoomAtViewportCenter(targetZoom, snapToLandmarks = false)
    if (changed) currentConfig.onZoomSnap()
    return changed
  }

  fun toggleIndicatorZoomAtViewportCenter(): EditorZoomLandmark? {
    val currentConfig = config ?: return null
    val targetZoom = currentConfig.zoomController.resolveIndicatorToggleTarget() ?: return null
    if (!setZoomAtViewportCenter(targetZoom)) return null
    val landmark = currentConfig.zoomController.resolveLandmark() ?: return null
    currentConfig.onZoomSnap()
    return landmark
  }

  private fun releaseForDirectPan() {
    val currentConfig = config?.takeIf { it.isUsable }
    val seed = currentConfig?.createMotionSeed()
    clearDirectSessions()
    if (currentConfig != null && seed != null) {
      finishOrRecover(config = currentConfig, seed = seed, allowConcurrentPan = true)
    } else {
      finishInteraction(currentConfig)
    }
  }

  fun interruptForDirectPan() {
    if (pinchSession != null || indirectZoomActive) {
      releaseForDirectPan()
      return
    }
    val currentConfig = config?.takeIf { it.isUsable } ?: return
    val active = activeMotion ?: return
    active.allowConcurrentPan = true
    endTransform(currentConfig)
  }

  fun cancel() {
    if (pinchSession == null && !indirectZoomActive) return
    val currentConfig = config?.takeIf { it.isUsable }
    val seed = currentConfig?.createMotionSeed()
    clearDirectSessions()
    stopMotion(commitRender = false)
    if (currentConfig != null && seed != null) {
      finishOrRecover(config = currentConfig, seed = seed)
    } else {
      settleAndFinishInteraction(currentConfig)
    }
  }

  fun reset() {
    abortForConfigurationChange(config)
    config = null
  }

  private val hasInteraction: Boolean
    get() = pinchSession != null || indirectZoomActive || activeMotion != null || transformActive

  private fun beginTransform(config: EditorViewportZoomSemanticConfig) {
    if (!transformActive) {
      config.viewportState.beginTransform()
      transformActive = true
    }
  }

  private fun endTransform(config: EditorViewportZoomSemanticConfig) {
    if (transformActive) {
      config.viewportState.endTransform()
      transformActive = false
    }
  }

  private fun clearDirectSessions() {
    pinchSession = null
    indirectZoomActive = false
    indirectSession = null
  }

  private fun setGestureZoom(
    config: EditorViewportZoomSemanticConfig,
    rawZoom: Float,
    previousRawZoom: Float,
    previousTimestampMillis: Long?,
    timestampMillis: Long,
  ): Float {
    val snapCandidate =
      clampDocumentLayoutZoom(
        zoom = rawZoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
    val bounds = computeDocumentZoomBounds(config.layoutSpec)
    val snapCandidateKey =
      if (rawZoom in bounds) config.zoomController.resolveSnapKey(snapCandidate) else null
    val rawVelocity =
      instantaneousLogZoomVelocity(
        previousZoom = previousRawZoom,
        zoom = rawZoom,
        previousTimestampMillis = previousTimestampMillis,
        timestampMillis = timestampMillis,
      )
    val nextSnap = snapCandidateKey?.takeIf {
      it == lastSnapKey || abs(rawVelocity) < DirectSnapVelocityThreshold
    }
    config.zoomController.setGestureDisplayZoom(
      rawZoom = rawZoom,
      snapZoom = snapCandidate.takeIf { nextSnap != null },
      layoutSpec = config.layoutSpec,
      viewportWidth = config.viewportWidth,
    )
    maybeSendZoomSnapHaptic(
      previousSnap = lastSnapKey,
      nextSnap = nextSnap,
      haptic = config.onZoomSnap,
    )
    lastSnapKey = nextSnap
    return config.zoomController.displayZoom
  }

  private fun finishOrRecover(
    config: EditorViewportZoomSemanticConfig,
    seed: MotionSeed,
    allowConcurrentPan: Boolean = false,
  ) {
    stopMotion(commitRender = false)
    val durationScale = motionDurationScale()
    val bounds = computeDocumentZoomBounds(config.layoutSpec)
    val displayZoom = config.zoomController.displayZoom
    if (displayZoom in bounds) {
      settleReleasedSnap(config = config, seed = seed, displayZoom = displayZoom)
      finishInteraction(config)
      return
    }
    val motion = EditorZoomMotion(displayZoom = displayZoom, bounds = bounds)
    val active =
      ActiveMotion(
        configOwner = config.zoomController,
        layoutSpec = config.layoutSpec,
        anchor = seed.anchor,
        startScrollOffset = seed.scrollOffset,
        startAnchorDisplayPosition = seed.anchorDisplayPosition,
        lastAppliedScrollOffset = config.viewportState.scrollOffset,
        motion = motion,
        allowConcurrentPan = allowConcurrentPan,
      )
    activeMotion = active
    if (allowConcurrentPan) endTransform(config) else beginTransform(config)

    if (coroutineScope == null || durationScale == 0.0) {
      applyMotionFrame(active, motion.advance(EditorZoomMotionTuning.MaximumMotionSeconds))
      finishMotion(active)
      return
    }
    motionJob = coroutineScope.launch {
      var previousFrameNanos: Long? = null
      while (activeMotion === active) {
        val frameNanos = withFrameNanos { it }
        val elapsedSeconds =
          previousFrameNanos?.let { previous ->
            (frameNanos - previous) / 1_000_000_000.0 / durationScale
          } ?: (1.0 / 60.0 / durationScale)
        previousFrameNanos = frameNanos
        val frame = active.motion.advance(elapsedSeconds)
        if (!applyMotionFrame(active, frame) || frame.finished) break
      }
      finishMotion(active)
    }
  }

  private fun settleReleasedSnap(
    config: EditorViewportZoomSemanticConfig,
    seed: MotionSeed,
    displayZoom: Float,
  ) {
    val settledZoom =
      clampDocumentLayoutZoom(
        zoom = displayZoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
    val snapKey = config.zoomController.resolveSnapKey(settledZoom) ?: return
    val anchorDisplayPosition =
      config.resolveAnchorDisplayPosition(anchor = seed.anchor, displayZoom = settledZoom) ?: return
    if (
      !config.zoomController.setDisplayZoom(
        zoom = settledZoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
    ) {
      return
    }

    val targetScrollOffset =
      seed.scrollOffset + (anchorDisplayPosition - seed.anchorDisplayPosition)
    config.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = true,
    )
    config.onAttachViewportAnchor(seed.anchor, anchorDisplayPosition, targetScrollOffset)
    lastSnapKey = snapKey
    config.onZoomSnap()
  }

  private fun applyMotionFrame(active: ActiveMotion, frame: EditorZoomMotionFrame): Boolean {
    val currentConfig = config?.takeIf { it.isUsable } ?: return false
    if (
      activeMotion !== active ||
        currentConfig.zoomController !== active.configOwner ||
        currentConfig.layoutSpec != active.layoutSpec
    ) {
      return false
    }
    if (active.allowConcurrentPan) {
      active.startScrollOffset +=
        currentConfig.viewportState.scrollOffset - active.lastAppliedScrollOffset
    }
    val anchorDisplayPosition =
      currentConfig.resolveAnchorDisplayPosition(
        anchor = active.anchor,
        displayZoom = frame.displayZoom,
      ) ?: return false
    val targetScrollOffset =
      active.startScrollOffset + (anchorDisplayPosition - active.startAnchorDisplayPosition)
    val previousZoom = currentConfig.zoomController.displayZoom
    if (
      !currentConfig.zoomController.setMotionDisplayZoom(
        displayZoom = frame.displayZoom,
        layoutSpec = currentConfig.layoutSpec,
        viewportWidth = currentConfig.viewportWidth,
      ) && previousZoom != frame.displayZoom
    ) {
      return false
    }
    currentConfig.viewportState.scrollToTransformTarget(
      offset = targetScrollOffset,
      retainUntilMeasuredBounds = previousZoom != frame.displayZoom,
    )
    currentConfig.onAttachViewportAnchor(active.anchor, anchorDisplayPosition, targetScrollOffset)
    active.lastAppliedScrollOffset = currentConfig.viewportState.scrollOffset
    val nextSnap = currentConfig.zoomController.resolveSnapKey(frame.displayZoom)
    if (frame.finished) {
      currentConfig.onZoomSnap()
    } else {
      maybeSendZoomSnapHaptic(
        previousSnap = lastSnapKey,
        nextSnap = nextSnap,
        haptic = currentConfig.onZoomSnap,
      )
    }
    lastSnapKey = nextSnap
    return true
  }

  private fun finishMotion(active: ActiveMotion) {
    if (activeMotion !== active) return
    activeMotion = null
    motionJob = null
    settleAndFinishInteraction(config)
  }

  private fun stopMotion(commitRender: Boolean) {
    activeMotion = null
    motionJob?.cancel()
    motionJob = null
    if (commitRender) config?.zoomController?.commitRenderZoom()
  }

  private fun finishInteraction(config: EditorViewportZoomSemanticConfig?) {
    config?.zoomController?.commitRenderZoom()
    if (config != null) endTransform(config) else transformActive = false
  }

  private fun settleAndFinishInteraction(config: EditorViewportZoomSemanticConfig?) {
    config
      ?.zoomController
      ?.setDisplayZoom(
        zoom = config.zoomController.displayZoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
    finishInteraction(config)
  }

  private fun abortForConfigurationChange(config: EditorViewportZoomSemanticConfig?) {
    clearDirectSessions()
    stopMotion(commitRender = false)
    if (config?.isUsable == true) {
      config.zoomController.setDisplayZoom(
        zoom = config.zoomController.displayZoom,
        layoutSpec = config.layoutSpec,
        viewportWidth = config.viewportWidth,
      )
      config.zoomController.commitRenderZoom()
      endTransform(config)
    } else {
      transformActive = false
    }
  }

  private fun motionDurationScale(): Double {
    val scale = coroutineScope?.coroutineContext?.get(MotionDurationScale)?.scaleFactor ?: 1f
    return scale.takeIf { it.isFinite() && it >= 0f }?.toDouble() ?: 1.0
  }

  private fun EditorViewportZoomSemanticConfig.createMotionSeed(): MotionSeed? {
    val anchor =
      when {
        pinchSession != null -> {
          val session = pinchSession ?: return null
          session.anchor
        }
        indirectSession != null -> {
          val session = indirectSession ?: return null
          session.anchor
        }
        else -> return null
      }
    val anchorDisplayPosition =
      resolveAnchorDisplayPosition(anchor, zoomController.displayZoom) ?: return null
    return MotionSeed(
      anchor = anchor,
      scrollOffset = viewportState.effectiveTransformScrollTarget,
      anchorDisplayPosition = anchorDisplayPosition,
    )
  }
}

private data class PinchSession(
  val anchor: EditorViewportAnchor,
  val startSample: EditorPinchSample,
  val startScrollOffset: Offset,
  val startRawZoom: Float,
  val startAnchorDisplayPosition: Offset,
  val startFocal: Offset,
  val pageSizes: List<PageSize>,
  val lastSample: EditorPinchSample,
  val rawZoom: Float,
)

private data class IndirectSession(
  val anchor: EditorViewportAnchor,
  val startScrollOffset: Offset,
  val startAnchorDisplayPosition: Offset,
  val startFocal: Offset,
  val pageSizes: List<PageSize>,
  val lastTimestampMillis: Long,
  val rawZoom: Float,
)

private data class MotionSeed(
  val anchor: EditorViewportAnchor,
  val scrollOffset: Offset,
  val anchorDisplayPosition: Offset,
)

private data class ActiveMotion(
  val configOwner: EditorZoomController,
  val layoutSpec: EditorDocumentLayoutSpec,
  val anchor: EditorViewportAnchor,
  var startScrollOffset: Offset,
  val startAnchorDisplayPosition: Offset,
  var lastAppliedScrollOffset: Offset,
  val motion: EditorZoomMotion,
  var allowConcurrentPan: Boolean,
)

private fun EditorViewportZoomSemanticConfig.rebasePinchSession(
  session: PinchSession
): PinchSession? {
  val anchor = resolveAnchor(session.lastSample.focalInRootPx) ?: return null
  val displayZoom = zoomController.displayZoom
  val anchorDisplayPosition = resolveAnchorDisplayPosition(anchor, displayZoom) ?: return null
  return PinchSession(
    anchor = anchor,
    startSample = session.lastSample,
    startScrollOffset = viewportState.effectiveTransformScrollTarget,
    startRawZoom = displayZoom,
    startAnchorDisplayPosition = anchorDisplayPosition,
    startFocal = toRootDp(session.lastSample.focalInRootPx),
    pageSizes = pageSizes,
    lastSample = session.lastSample,
    rawZoom = displayZoom,
  )
}

private fun instantaneousLogZoomVelocity(
  previousZoom: Float,
  zoom: Float,
  previousTimestampMillis: Long?,
  timestampMillis: Long,
): Double {
  if (!previousZoom.isFinite() || previousZoom <= 0f || !zoom.isFinite() || zoom <= 0f) {
    return Double.POSITIVE_INFINITY
  }
  if (previousZoom == zoom) return 0.0
  if (previousTimestampMillis == null) return Double.POSITIVE_INFINITY
  val elapsedSeconds = (timestampMillis - previousTimestampMillis) / 1000.0
  return if (elapsedSeconds.isFinite() && elapsedSeconds > 0.0) {
    ln(zoom.toDouble() / previousZoom) / elapsedSeconds
  } else {
    Double.POSITIVE_INFINITY
  }
}

private fun EditorViewportZoomSemanticConfig.createIndirectSession(
  focalInRootPx: Offset,
  timestampMillis: Long,
): IndirectSession? {
  val viewportState = viewportState
  val effectiveScrollTarget = viewportState.effectiveTransformScrollTarget
  val unappliedScrollDelta = effectiveScrollTarget - viewportState.scrollOffset
  val editorRect = uiState.editorRectInRoot() ?: return null
  val focalInEditor = toEditorDp(focalInRootPx - editorRect.topLeft) + unappliedScrollDelta
  val displayZoom = zoomController.displayZoom
  val anchor =
    resolveViewportTransform(displayZoom = displayZoom)
      .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y) ?: return null
  val anchorDisplayPosition = resolveAnchorDisplayPosition(anchor, displayZoom) ?: return null
  val focal = toRootDp(focalInRootPx)
  return IndirectSession(
    anchor = anchor,
    startScrollOffset = effectiveScrollTarget,
    startAnchorDisplayPosition = anchorDisplayPosition,
    startFocal = focal,
    pageSizes = pageSizes,
    lastTimestampMillis = timestampMillis,
    rawZoom = displayZoom,
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

private fun isValidScaleUpdate(focalInRootPx: Offset, scaleFactor: Float): Boolean =
  focalInRootPx.x.isFinite() &&
    focalInRootPx.y.isFinite() &&
    scaleFactor.isFinite() &&
    scaleFactor > 0f

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
    if (displayZoom == null) transform else transform.copy(displayZoom = displayZoom)
  }

private fun EditorViewportZoomSemanticConfig.toEditorDp(focalPx: Offset): Offset =
  Offset(x = focalPx.x / density, y = focalPx.y / density)

private fun EditorViewportZoomSemanticConfig.toRootDp(focalInRootPx: Offset): Offset =
  Offset(x = focalInRootPx.x / density, y = focalInRootPx.y / density)

private fun maybeSendZoomSnapHaptic(
  previousSnap: EditorZoomSnapKey?,
  nextSnap: EditorZoomSnapKey?,
  haptic: () -> Unit,
) {
  if (nextSnap != null && nextSnap != previousSnap) haptic()
}
