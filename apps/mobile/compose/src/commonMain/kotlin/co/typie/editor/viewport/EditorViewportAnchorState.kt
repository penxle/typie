package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.VerticalSpan
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveKeepVisibleRange
import co.typie.editor.scroll.resolveKeepVisibleScrollOffset

internal data class EditorViewportAnchorGeometry(
  val pointY: Float,
  val pointX: Float = 0f,
  val rect: VerticalSpan? = null,
)

internal data class EditorViewportAnchorRevealOrigin(
  val scrollY: Float,
  val target: EditorBringIntoViewTarget,
  val policy: EditorBringIntoViewPolicy,
)

internal class EditorViewportAnchorState {
  private data class Active(
    val identity: ViewportAnchor,
    val pointAttachmentX: Float,
    val pointAttachmentY: Float,
    val rect: VerticalSpan?,
    val revealOrigin: EditorViewportAnchorRevealOrigin?,
  )

  private data class CapturedViewportIdentity(
    val editor: Editor,
    val revision: Long,
    val point: ViewportAnchorPoint,
    val identity: ViewportAnchor,
  )

  private var active: Active? = null
  private var preferredSelection: ViewportAnchor? = null
  private var capturedViewportIdentity: CapturedViewportIdentity? = null
  private var observedScrollRevision: Int? = null
  private var observedVisibleTop: Float? = null
  private var observedVisibleBottom: Float? = null

  val identity: ViewportAnchor?
    get() = active?.identity

  val pointAttachmentY: Float?
    get() = active?.pointAttachmentY

  val pointAttachmentX: Float?
    get() = active?.pointAttachmentX

  val preferredSelectionIdentity: ViewportAnchor?
    get() = preferredSelection

  fun clear() {
    active = null
    preferredSelection = null
    capturedViewportIdentity = null
  }

  fun captureViewportIdentity(
    editor: Editor,
    revision: Long,
    point: ViewportAnchorPoint,
  ): ViewportAnchor? {
    capturedViewportIdentity?.let { captured ->
      if (captured.editor === editor && captured.revision == revision && captured.point == point) {
        return captured.identity
      }
    }
    val identity = editor.captureViewportAnchorAt(revision, point)?.identity ?: return null
    capturedViewportIdentity =
      CapturedViewportIdentity(
        editor = editor,
        revision = revision,
        point = point,
        identity = identity,
      )
    return identity
  }

  fun needsSelectionAdoption(identity: ViewportAnchor): Boolean = preferredSelection != identity

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

  fun attach(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
  ) {
    attachActive(identity, geometry, scrollOffset)
  }

  fun attachSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    revealOrigin: EditorViewportAnchorRevealOrigin? = null,
  ) {
    preferredSelection = identity
    attachActive(identity, geometry, scrollOffset, revealOrigin)
  }

  fun attachViewport(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
  ) {
    attachActive(identity, geometry, scrollOffset)
  }

  fun adoptSelection(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    visibleArea: EditorVisibleArea,
    preserveActiveAnchor: Boolean,
  ) {
    if (!needsSelectionAdoption(identity)) return
    val activate =
      !preserveActiveAnchor &&
        (active != null || canRetainAfterDirectScroll(geometry, scrollOffset.y, visibleArea))
    preferredSelection = identity
    if (activate) attachActive(identity, geometry, scrollOffset)
  }

  fun clearPreferredSelection() {
    preferredSelection = null
  }

  fun tryReactivatePreferredSelection(
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    val identity = preferredSelection ?: return false
    val rect = geometry.rect ?: return false
    val guard = resolveKeepVisibleRange(visibleArea)
    if (!guard.isValid) return false
    if (!rect.top.isFinite() || !rect.bottom.isFinite() || rect.bottom < rect.top) return false
    if (rect.top - scrollOffset.y < guard.top || rect.bottom - scrollOffset.y > guard.bottom)
      return false
    attachActive(identity, geometry, scrollOffset)
    return true
  }

  private fun attachActive(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    revealOrigin: EditorViewportAnchorRevealOrigin? = null,
  ) {
    if (!geometry.pointX.isFinite() || !geometry.pointY.isFinite()) return
    if (!scrollOffset.x.isFinite() || !scrollOffset.y.isFinite()) return
    active =
      Active(
        identity = identity,
        pointAttachmentX = geometry.pointX - scrollOffset.x,
        pointAttachmentY = geometry.pointY - scrollOffset.y,
        rect = geometry.rect,
        revealOrigin = revealOrigin,
      )
  }

  fun publicationScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollOffset: Offset,
    maximumScrollOffset: Offset,
  ): Offset {
    val current = active ?: return currentScrollOffset
    if (!geometry.pointX.isFinite() || !geometry.pointY.isFinite()) return currentScrollOffset
    if (!maximumScrollOffset.x.isFinite() || !maximumScrollOffset.y.isFinite()) {
      return currentScrollOffset
    }
    if (maximumScrollOffset.x < 0f || maximumScrollOffset.y < 0f) return currentScrollOffset
    return Offset(
      x = (geometry.pointX - current.pointAttachmentX).coerceIn(0f, maximumScrollOffset.x),
      y = (geometry.pointY - current.pointAttachmentY).coerceIn(0f, maximumScrollOffset.y),
    )
  }

  fun publicationRevealScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollOffset: Offset,
    maximumScrollOffset: Offset,
    visibleArea: EditorVisibleArea,
    resolveReveal: ((EditorViewportAnchorRevealOrigin) -> Float?)? = null,
  ): Offset {
    val exact = publicationScroll(geometry, currentScrollOffset, maximumScrollOffset)
    if (!rectHeightChanged(active?.rect, geometry.rect)) return exact
    active?.revealOrigin?.let { origin ->
      resolveReveal?.invoke(origin)?.let {
        return exact.copy(y = it.coerceIn(0f, maximumScrollOffset.y))
      }
    }
    return exact.copy(y = resizeScroll(geometry, exact.y, maximumScrollOffset.y, visibleArea))
  }

  fun acceptGeometry(geometry: EditorViewportAnchorGeometry, scrollOffset: Offset) {
    val current = active ?: return
    attachActive(
      identity = current.identity,
      geometry = geometry,
      scrollOffset = scrollOffset,
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
