package co.typie.screen.editor.editor.layout

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PublishedBundle
import co.typie.editor.ffi.ViewportAnchorResolution
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.viewport.EditorViewportAnchorGeometry
import co.typie.editor.viewport.EditorViewportAnchorRevealOrigin
import co.typie.editor.viewport.EditorViewportAnchorState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveViewportAnchorContentOriginY
import co.typie.editor.viewport.toEditorViewportAnchorGeometry
import co.typie.editor.viewport.viewportCenterAnchorPoint

internal sealed interface EditorViewportAnchorPublication {
  data object Withhold : EditorViewportAnchorPublication

  data class Ready(val scrollY: Float, val geometry: EditorViewportAnchorGeometry?) :
    EditorViewportAnchorPublication
}

internal fun reconcileViewportAnchorPublication(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  publishedBundle: PublishedBundle?,
  candidateState: EditorState,
  measuredScrollFrame: EditorScrollFrame,
  currentScrollOffset: Offset,
  maximumScrollY: Float,
  contentOriginY: Float,
  smoothRevealActive: Boolean = false,
): EditorViewportAnchorPublication {
  val currentScrollY = currentScrollOffset.y
  val candidateFrame = measuredScrollFrame.withState(candidateState)
  val selectionCapture = editor.captureSelectionViewportAnchor(candidateState.version)
  if (selectionCapture == null && candidateState.selection != null) {
    return EditorViewportAnchorPublication.Withhold
  }
  if (selectionCapture != null && anchorState.needsSelectionAdoption(selectionCapture.identity)) {
    val geometry =
      selectionCapture.geometry.toEditorViewportAnchorGeometry(
        frame = candidateFrame,
        contentOriginY = contentOriginY,
      ) ?: return EditorViewportAnchorPublication.Withhold
    anchorState.adoptSelection(
      identity = selectionCapture.identity,
      geometry = geometry,
      scrollY = currentScrollY,
      visibleArea = candidateFrame.visibleArea,
      preserveActiveAnchor = smoothRevealActive,
    )
  } else if (candidateState.selection == null && anchorState.preferredSelectionIdentity != null) {
    if (!smoothRevealActive) {
      val publishedState = publishedBundle?.snapshot ?: candidateState
      val publishedFrame = candidateFrame.withState(publishedState)
      attachViewportCenterAnchor(
        editor = editor,
        anchorState = anchorState,
        revision = publishedState.version,
        frame = publishedFrame,
        scrollOffset = currentScrollOffset,
        contentOriginY = resolveViewportAnchorContentOriginY(publishedFrame),
      ) ?: return EditorViewportAnchorPublication.Withhold
    }
    anchorState.clearPreferredSelection()
  }
  val unanchoredPublication =
    EditorViewportAnchorPublication.Ready(
      scrollY = currentScrollY.coerceIn(0f, maximumScrollY),
      geometry = null,
    )
  if (publishedBundle == null || publishedBundle.snapshot.version == candidateState.version) {
    return unanchoredPublication
  }
  val resolution =
    resolveCandidateViewportAnchor(
      editor = editor,
      anchorState = anchorState,
      publishedBundle = publishedBundle,
      candidateFrame = candidateFrame,
      currentScrollOffset = currentScrollOffset,
    ) ?: return unanchoredPublication

  return when (resolution) {
    ViewportAnchorResolution.Unavailable -> EditorViewportAnchorPublication.Withhold
    ViewportAnchorResolution.Deleted,
    ViewportAnchorResolution.NotLaidOut -> {
      anchorState.clear()
      unanchoredPublication
    }
    is ViewportAnchorResolution.Resolved -> {
      val geometry =
        resolution.geometry.toEditorViewportAnchorGeometry(
          frame = candidateFrame,
          contentOriginY = contentOriginY,
        ) ?: return EditorViewportAnchorPublication.Withhold
      EditorViewportAnchorPublication.Ready(
        scrollY =
          anchorState.publicationRevealScroll(
            geometry = geometry,
            currentScrollY = currentScrollY,
            maximumScrollY = maximumScrollY,
            visibleArea = measuredScrollFrame.visibleArea,
            resolveReveal = { origin ->
              when (
                val result =
                  resolveEditorScrollIntent(
                    frame = candidateFrame,
                    target = origin.target,
                    policy = origin.policy,
                    currentScroll = origin.scrollY,
                    contentOriginY = contentOriginY,
                    maximumScrollY = maximumScrollY,
                  )
              ) {
                EditorScrollIntentResult.Unresolved -> null
                EditorScrollIntentResult.NoScroll -> origin.scrollY
                is EditorScrollIntentResult.ScrollTo -> result.y
              }
            },
          ),
        geometry = geometry,
      )
    }
  }
}

private fun resolveCandidateViewportAnchor(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  publishedBundle: PublishedBundle,
  candidateFrame: EditorScrollFrame,
  currentScrollOffset: Offset,
): ViewportAnchorResolution? {
  val identity = anchorState.identity ?: return null
  val resolution = editor.resolveViewportAnchor(candidateFrame.state.version, identity)
  if (
    resolution !is ViewportAnchorResolution.Deleted &&
      resolution !is ViewportAnchorResolution.NotLaidOut
  ) {
    return resolution
  }

  val publishedFrame = candidateFrame.withState(publishedBundle.snapshot)
  val publishedContentOriginY = resolveViewportAnchorContentOriginY(publishedFrame)
  val centerPoint =
    viewportCenterAnchorPoint(publishedFrame, currentScrollOffset, publishedContentOriginY)
  val fallback = centerPoint?.let {
    editor.captureViewportAnchorAt(publishedBundle.snapshot.version, it)
  }
  val oldGeometry =
    fallback
      ?.geometry
      ?.toEditorViewportAnchorGeometry(
        frame = publishedFrame,
        contentOriginY = publishedContentOriginY,
      )
  if (fallback == null || oldGeometry == null) {
    anchorState.clear()
    return null
  }
  anchorState.attach(fallback.identity, oldGeometry, currentScrollOffset.y)
  return editor.resolveViewportAnchor(candidateFrame.state.version, fallback.identity)
}

internal fun reconcileViewportAnchorObservation(
  editor: Editor?,
  anchorState: EditorViewportAnchorState,
  bundle: PublishedBundle?,
  frame: EditorScrollFrame,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  mode: EditorViewportScrollReconcileMode,
  smoothRevealActive: Boolean,
  handoffTarget: EditorBringIntoViewTarget?,
  selectionRevealOrigin: EditorViewportAnchorRevealOrigin?,
  contentOriginY: Float,
) {
  if (mode == EditorViewportScrollReconcileMode.Disabled || editor == null || bundle == null) {
    anchorState.clear()
    return
  }
  if (viewportState.isTransforming) return
  val scrollChanged = anchorState.consumeScrollChange(viewportState.lastScrollRevision)
  val visibleAreaChanged = anchorState.consumeVisibleAreaChange(visibleArea)

  val revision = bundle.snapshot.version
  val presentationFrame = frame.withState(bundle.snapshot)
  var geometry =
    if (handoffTarget != null) {
      attachSelectionViewportAnchor(
        editor = editor,
        anchorState = anchorState,
        revision = revision,
        frame = presentationFrame,
        scrollY = viewportState.scrollOffset.y,
        visibleArea = visibleArea,
        requireGuard = handoffTarget != EditorBringIntoViewTarget.CurrentSelectionHead,
        revealOrigin = selectionRevealOrigin,
        contentOriginY = contentOriginY,
      )
    } else if (anchorState.identity == null) {
      attachSelectionViewportAnchor(
        editor = editor,
        anchorState = anchorState,
        revision = revision,
        frame = presentationFrame,
        scrollY = viewportState.scrollOffset.y,
        visibleArea = visibleArea,
        requireGuard = true,
        contentOriginY = contentOriginY,
      )
        ?: attachViewportCenterAnchor(
          editor = editor,
          anchorState = anchorState,
          revision = revision,
          frame = presentationFrame,
          scrollOffset = viewportState.scrollOffset,
          contentOriginY = contentOriginY,
        )
    } else {
      null
    }

  if (
    scrollChanged &&
      handoffTarget == null &&
      !smoothRevealActive &&
      !viewportState.lastScrollWasAuto
  ) {
    anchorState.finishRevealConvergence()
  }
  if (smoothRevealActive) {
    if (scrollChanged && handoffTarget == null) {
      attachViewportCenterAnchor(
        editor,
        anchorState,
        revision,
        presentationFrame,
        viewportState.scrollOffset,
        contentOriginY,
      )
    }
    return
  }
  if (!scrollChanged && !visibleAreaChanged) return

  geometry =
    geometry
      ?: resolveActiveViewportAnchorGeometry(
        editor,
        anchorState,
        revision,
        presentationFrame,
        contentOriginY,
      )
  if (scrollChanged && handoffTarget == null) {
    val preferredSelectionGeometry =
      if (!viewportState.lastScrollWasAuto) {
        resolvePreferredSelectionViewportAnchorGeometry(
          editor = editor,
          anchorState = anchorState,
          revision = revision,
          frame = presentationFrame,
          contentOriginY = contentOriginY,
        )
      } else {
        null
      }
    when {
      viewportState.lastScrollWasAuto ->
        geometry?.let { anchorState.acceptGeometry(it, viewportState.scrollOffset.y) }
      preferredSelectionGeometry != null &&
        anchorState.tryReactivatePreferredSelection(
          geometry = preferredSelectionGeometry,
          scrollY = viewportState.scrollOffset.y,
          visibleArea = visibleArea,
        ) -> geometry = preferredSelectionGeometry
      geometry != null &&
        anchorState.canRetainAfterDirectScroll(
          geometry = geometry,
          scrollY = viewportState.scrollOffset.y,
          visibleArea = visibleArea,
        ) -> anchorState.acceptGeometry(geometry, viewportState.scrollOffset.y)
      else -> {
        geometry =
          attachViewportCenterAnchor(
            editor,
            anchorState,
            revision,
            presentationFrame,
            viewportState.scrollOffset,
            contentOriginY,
          )
      }
    }
  }

  if (visibleAreaChanged && geometry != null) {
    val targetY =
      anchorState.resizeScroll(
        geometry = geometry,
        currentScrollY = viewportState.scrollOffset.y,
        maximumScrollY = viewportState.maxScrollY,
        visibleArea = visibleArea,
      )
    viewportState.scrollToY(targetY = targetY, isAutoScroll = true)
    anchorState.acceptGeometry(geometry, viewportState.scrollOffset.y)
  }
}

internal fun attachSelectionViewportAnchor(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
  scrollY: Float,
  visibleArea: EditorVisibleArea,
  requireGuard: Boolean,
  revealOrigin: EditorViewportAnchorRevealOrigin? = null,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  val capture = editor.captureSelectionViewportAnchor(revision) ?: return null
  val geometry =
    capture.geometry.toEditorViewportAnchorGeometry(frame = frame, contentOriginY = contentOriginY)
      ?: return null
  if (requireGuard && !anchorState.canRetainAfterDirectScroll(geometry, scrollY, visibleArea)) {
    return null
  }
  anchorState.attachSelection(capture.identity, geometry, scrollY, revealOrigin)
  return geometry
}

internal fun attachViewportCenterAnchor(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
  scrollOffset: Offset,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  val point = viewportCenterAnchorPoint(frame, scrollOffset, contentOriginY) ?: return null
  val capture = editor.captureViewportAnchorAt(revision, point) ?: return null
  val geometry =
    capture.geometry.toEditorViewportAnchorGeometry(frame = frame, contentOriginY = contentOriginY)
      ?: return null
  anchorState.attachViewport(capture.identity, geometry, scrollOffset.y)
  return geometry
}

private fun resolvePreferredSelectionViewportAnchorGeometry(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  val identity = anchorState.preferredSelectionIdentity ?: return null
  return when (val resolution = editor.resolveViewportAnchor(revision, identity)) {
    ViewportAnchorResolution.Deleted -> {
      anchorState.clearPreferredSelection()
      null
    }
    ViewportAnchorResolution.NotLaidOut,
    ViewportAnchorResolution.Unavailable -> null
    is ViewportAnchorResolution.Resolved ->
      resolution.geometry.toEditorViewportAnchorGeometry(frame, contentOriginY)
  }
}

private fun resolveActiveViewportAnchorGeometry(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  val identity = anchorState.identity ?: return null
  return ((editor.resolveViewportAnchor(revision, identity) as? ViewportAnchorResolution.Resolved)
      ?.geometry)
    ?.toEditorViewportAnchorGeometry(frame, contentOriginY)
}
