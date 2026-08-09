package co.typie.editor.viewport

import co.typie.editor.VerticalSpan
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveKeepVisibleRange
import co.typie.editor.scroll.resolveKeepVisibleScrollOffset

internal data class EditorViewportAnchorGeometry(val pointY: Float, val rect: VerticalSpan? = null)

internal data class EditorViewportAnchorRevealOrigin(
  val scrollY: Float,
  val target: EditorBringIntoViewTarget,
  val policy: EditorBringIntoViewPolicy,
)

internal class EditorViewportAnchorState {
  private data class Active(
    val identity: ViewportAnchor,
    val pointAttachmentY: Float,
    val rect: VerticalSpan?,
    val revealOrigin: EditorViewportAnchorRevealOrigin?,
  )

  private var active: Active? = null
  private var preferredSelection: ViewportAnchor? = null
  private var observedScrollRevision: Int? = null
  private var observedVisibleTop: Float? = null
  private var observedVisibleBottom: Float? = null

  val identity: ViewportAnchor?
    get() = active?.identity

  val pointAttachmentY: Float?
    get() = active?.pointAttachmentY

  val preferredSelectionIdentity: ViewportAnchor?
    get() = preferredSelection

  fun clear() {
    active = null
    preferredSelection = null
  }

  fun consumeScrollChange(revision: Int): Boolean {
    val previous = observedScrollRevision
    observedScrollRevision = revision
    return previous != null && previous != revision
  }

  fun consumeVisibleAreaChange(visibleArea: EditorVisibleArea): Boolean {
    val top = visibleArea.visibleViewportTop
    val bottom = visibleArea.visibleViewportBottom
    val changed =
      observedVisibleTop != null && (observedVisibleTop != top || observedVisibleBottom != bottom)
    observedVisibleTop = top
    observedVisibleBottom = bottom
    return changed
  }

  fun attach(identity: ViewportAnchor, geometry: EditorViewportAnchorGeometry, scrollY: Float) {
    attachActive(identity, geometry, scrollY)
  }

  fun attachSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollY: Float,
    revealOrigin: EditorViewportAnchorRevealOrigin? = null,
  ) {
    preferredSelection = identity
    attachActive(identity, geometry, scrollY, revealOrigin)
  }

  fun attachViewport(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollY: Float,
  ) {
    attachActive(identity, geometry, scrollY)
  }

  fun clearPreferredSelection() {
    preferredSelection = null
  }

  fun tryReactivatePreferredSelection(
    geometry: EditorViewportAnchorGeometry,
    scrollY: Float,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    val identity = preferredSelection ?: return false
    val rect = geometry.rect ?: return false
    val guard = resolveKeepVisibleRange(visibleArea)
    if (
      !guard.isValid ||
        !rect.top.isFinite() ||
        !rect.bottom.isFinite() ||
        rect.bottom < rect.top ||
        rect.top - scrollY < guard.top ||
        rect.bottom - scrollY > guard.bottom
    ) {
      return false
    }
    attachActive(identity, geometry, scrollY)
    return true
  }

  private fun attachActive(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollY: Float,
    revealOrigin: EditorViewportAnchorRevealOrigin? = null,
  ) {
    if (!geometry.pointY.isFinite() || !scrollY.isFinite()) return
    active =
      Active(
        identity = identity,
        pointAttachmentY = geometry.pointY - scrollY,
        rect = geometry.rect,
        revealOrigin = revealOrigin,
      )
  }

  fun publicationScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollY: Float,
    maximumScrollY: Float,
  ): Float {
    val attachment = active?.pointAttachmentY ?: return currentScrollY
    if (!geometry.pointY.isFinite() || !maximumScrollY.isFinite() || maximumScrollY < 0f) {
      return currentScrollY
    }
    return (geometry.pointY - attachment).coerceIn(0f, maximumScrollY)
  }

  fun publicationRevealScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollY: Float,
    maximumScrollY: Float,
    visibleArea: EditorVisibleArea,
    resolveReveal: ((EditorViewportAnchorRevealOrigin) -> Float?)? = null,
  ): Float {
    val exact = publicationScroll(geometry, currentScrollY, maximumScrollY)
    if (!rectHeightChanged(active?.rect, geometry.rect)) return exact
    active?.revealOrigin?.let { origin ->
      resolveReveal?.invoke(origin)?.let {
        return it.coerceIn(0f, maximumScrollY)
      }
    }
    return resizeScroll(geometry, exact, maximumScrollY, visibleArea)
  }

  fun acceptGeometry(geometry: EditorViewportAnchorGeometry, scrollY: Float) {
    val current = active ?: return
    attachActive(
      identity = current.identity,
      geometry = geometry,
      scrollY = scrollY,
      revealOrigin = current.revealOrigin,
    )
  }

  fun finishRevealConvergence() {
    active = active?.copy(revealOrigin = null)
  }

  fun canRetainAfterDirectScroll(
    geometry: EditorViewportAnchorGeometry,
    scrollY: Float,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    val guard = resolveKeepVisibleRange(visibleArea)
    if (!guard.isValid) return false
    val span = geometry.guardedSpan(guard)
    val topInViewport = span.top - scrollY
    val bottomInViewport = span.bottom - scrollY
    return topInViewport >= guard.top && bottomInViewport <= guard.bottom
  }

  fun resizeScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollY: Float,
    maximumScrollY: Float,
    visibleArea: EditorVisibleArea,
  ): Float {
    if (canRetainAfterDirectScroll(geometry, currentScrollY, visibleArea)) {
      return currentScrollY
    }
    val guard = resolveKeepVisibleRange(visibleArea)
    val span = geometry.guardedSpan(guard)
    return resolveKeepVisibleScrollOffset(
      currentScroll = currentScrollY,
      targetTopInContent = span.top,
      targetBottomInContent = span.bottom,
      visibleArea = visibleArea,
      maximumScrollY = maximumScrollY,
    ) ?: currentScrollY
  }

  private fun EditorViewportAnchorGeometry.guardedSpan(guard: VerticalSpan): VerticalSpan {
    val candidate = rect
    return if (candidate != null && candidate.height <= guard.height) {
      candidate
    } else {
      VerticalSpan(top = pointY, bottom = pointY)
    }
  }

  private fun rectHeightChanged(previous: VerticalSpan?, current: VerticalSpan?): Boolean {
    if (current == null) return false
    if (previous == null) return true
    return kotlin.math.abs(current.height - previous.height) > 1f
  }
}
