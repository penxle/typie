package co.typie.screen.editor.editor.layout

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PublishedBundle
import co.typie.editor.ffi.ViewportAnchorResolution
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.viewport.EditorViewportAnchorGeometry
import co.typie.editor.viewport.EditorViewportAnchorRevealOrigin
import co.typie.editor.viewport.EditorViewportAnchorState
import co.typie.editor.viewport.EditorViewportState
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
  currentScrollY: Float,
  maximumScrollY: Float,
  contentOriginY: Float,
): EditorViewportAnchorPublication {
  if (publishedBundle == null || publishedBundle.snapshot.version == candidateState.version) {
    return EditorViewportAnchorPublication.Ready(scrollY = currentScrollY, geometry = null)
  }
  var identity =
    anchorState.identity
      ?: return EditorViewportAnchorPublication.Ready(scrollY = currentScrollY, geometry = null)
  val candidateFrame = measuredScrollFrame.withState(candidateState)

  var resolution = editor.resolveViewportAnchor(candidateState.version, identity)
  if (resolution is ViewportAnchorResolution.Deleted) {
    val publishedFrame = measuredScrollFrame.withState(publishedBundle.snapshot)
    val centerPoint = viewportCenterAnchorPoint(publishedFrame, currentScrollY)
    val fallback = centerPoint?.let {
      editor.captureViewportAnchorAt(publishedBundle.snapshot.version, it)
    }
    if (fallback == null) {
      anchorState.clear()
      return EditorViewportAnchorPublication.Ready(
        scrollY = currentScrollY.coerceIn(0f, maximumScrollY),
        geometry = null,
      )
    }
    val oldGeometry =
      (editor.resolveViewportAnchor(publishedBundle.snapshot.version, fallback)
          as? ViewportAnchorResolution.Resolved)
        ?.geometry
        ?.toEditorViewportAnchorGeometry(publishedFrame)
    if (oldGeometry == null) return EditorViewportAnchorPublication.Withhold
    anchorState.attach(fallback, oldGeometry, currentScrollY)
    identity = fallback
    resolution = editor.resolveViewportAnchor(candidateState.version, identity)
  }

  return when (resolution) {
    ViewportAnchorResolution.Unavailable -> EditorViewportAnchorPublication.Withhold
    ViewportAnchorResolution.Deleted -> {
      anchorState.clear()
      EditorViewportAnchorPublication.Ready(
        scrollY = currentScrollY.coerceIn(0f, maximumScrollY),
        geometry = null,
      )
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

internal fun reconcileViewportAnchorObservation(
  editor: Editor?,
  anchorState: EditorViewportAnchorState,
  bundle: PublishedBundle?,
  frame: EditorScrollFrame,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  mode: EditorViewportScrollReconcileMode,
  smoothRevealActive: Boolean,
  handoffToSelection: Boolean,
  selectionRevealOrigin: EditorViewportAnchorRevealOrigin?,
) {
  val scrollChanged = anchorState.consumeScrollChange(viewportState.lastScrollRevision)
  val visibleAreaChanged = anchorState.consumeVisibleAreaChange(visibleArea)
  if (mode == EditorViewportScrollReconcileMode.Disabled || editor == null || bundle == null) {
    anchorState.clear()
    return
  }
  if (viewportState.isTransforming) return

  val revision = bundle.snapshot.version
  val presentationFrame = frame.withState(bundle.snapshot)
  var geometry =
    if (handoffToSelection) {
      attachSelectionViewportAnchor(
        editor = editor,
        anchorState = anchorState,
        revision = revision,
        frame = presentationFrame,
        scrollY = viewportState.scrollOffset.y,
        visibleArea = visibleArea,
        requireGuard = false,
        revealOrigin = selectionRevealOrigin,
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
      )
        ?: attachViewportCenterAnchor(
          editor,
          anchorState,
          revision,
          presentationFrame,
          viewportState.scrollOffset.y,
        )
    } else {
      null
    }

  if (
    scrollChanged && !handoffToSelection && !smoothRevealActive && !viewportState.lastScrollWasAuto
  ) {
    anchorState.finishRevealConvergence()
  }
  if (smoothRevealActive) {
    if (scrollChanged && !handoffToSelection) {
      attachViewportCenterAnchor(
        editor,
        anchorState,
        revision,
        presentationFrame,
        viewportState.scrollOffset.y,
      )
    }
    return
  }
  if (!scrollChanged && !visibleAreaChanged) return

  geometry =
    geometry
      ?: resolveActiveViewportAnchorGeometry(editor, anchorState, revision, presentationFrame)
  if (scrollChanged && !handoffToSelection) {
    val preferredSelectionGeometry =
      if (!viewportState.lastScrollWasAuto) {
        resolvePreferredSelectionViewportAnchorGeometry(
          editor = editor,
          anchorState = anchorState,
          revision = revision,
          frame = presentationFrame,
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
            viewportState.scrollOffset.y,
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
): EditorViewportAnchorGeometry? {
  val identity = editor.captureSelectionViewportAnchor(revision) ?: return null
  val geometry =
    ((editor.resolveViewportAnchor(revision, identity) as? ViewportAnchorResolution.Resolved)
        ?.geometry)
      ?.toEditorViewportAnchorGeometry(frame) ?: return null
  if (requireGuard && !anchorState.canRetainAfterDirectScroll(geometry, scrollY, visibleArea)) {
    return null
  }
  anchorState.attachSelection(identity, geometry, scrollY, revealOrigin)
  return geometry
}

internal fun attachViewportCenterAnchor(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
  scrollY: Float,
): EditorViewportAnchorGeometry? {
  val point = viewportCenterAnchorPoint(frame, scrollY) ?: return null
  val identity = editor.captureViewportAnchorAt(revision, point) ?: return null
  val geometry =
    ((editor.resolveViewportAnchor(revision, identity) as? ViewportAnchorResolution.Resolved)
        ?.geometry)
      ?.toEditorViewportAnchorGeometry(frame) ?: return null
  anchorState.attachViewport(identity, geometry, scrollY)
  return geometry
}

private fun resolvePreferredSelectionViewportAnchorGeometry(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
): EditorViewportAnchorGeometry? {
  val identity = anchorState.preferredSelectionIdentity ?: return null
  return when (val resolution = editor.resolveViewportAnchor(revision, identity)) {
    ViewportAnchorResolution.Deleted -> {
      anchorState.clearPreferredSelection()
      null
    }
    ViewportAnchorResolution.Unavailable -> null
    is ViewportAnchorResolution.Resolved ->
      resolution.geometry.toEditorViewportAnchorGeometry(frame)
  }
}

private fun resolveActiveViewportAnchorGeometry(
  editor: Editor,
  anchorState: EditorViewportAnchorState,
  revision: Long,
  frame: EditorScrollFrame,
): EditorViewportAnchorGeometry? {
  val identity = anchorState.identity ?: return null
  return ((editor.resolveViewportAnchor(revision, identity) as? ViewportAnchorResolution.Resolved)
      ?.geometry)
    ?.toEditorViewportAnchorGeometry(frame)
}
