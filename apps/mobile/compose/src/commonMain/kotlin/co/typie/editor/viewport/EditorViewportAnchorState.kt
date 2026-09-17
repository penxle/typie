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
import kotlin.math.abs

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

internal data class EditorViewportAnchorScroll(
  val scrollOffset: Offset,
  val attachmentAchieved: Boolean,
)

internal class EditorViewportAnchorState {
  private enum class Source {
    Selection,
    Viewport,
  }

  private data class Active(
    val identity: ViewportAnchor,
    val source: Source,
    val geometry: EditorViewportAnchorGeometry,
    val pointAttachmentX: Float,
    val pointAttachmentY: Float,
    val attachmentPending: Boolean,
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
    attachActive(identity, geometry, scrollOffset, revealOrigin, source = Source.Selection)
  }

  fun attachViewport(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    attachmentPending: Boolean = false,
  ) {
    attachActive(identity, geometry, scrollOffset, attachmentPending = attachmentPending)
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
    if (activate) attachActive(identity, geometry, scrollOffset, source = Source.Selection)
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
    attachActive(identity, geometry, scrollOffset, source = Source.Selection)
    return true
  }

  private fun attachActive(
    identity: ViewportAnchor,
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    revealOrigin: EditorViewportAnchorRevealOrigin? = null,
    attachmentPending: Boolean = false,
    source: Source = Source.Viewport,
  ) {
    if (!geometry.pointX.isFinite() || !geometry.pointY.isFinite()) return
    if (!scrollOffset.x.isFinite() || !scrollOffset.y.isFinite()) return
    active =
      Active(
        identity = identity,
        source = source,
        geometry = geometry,
        pointAttachmentX = geometry.pointX - scrollOffset.x,
        pointAttachmentY = geometry.pointY - scrollOffset.y,
        attachmentPending = attachmentPending,
        revealOrigin = revealOrigin,
      )
  }

  fun publicationScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollOffset: Offset,
    maximumScrollOffset: Offset,
  ): EditorViewportAnchorScroll {
    val current =
      active ?: return EditorViewportAnchorScroll(currentScrollOffset, attachmentAchieved = false)
    if (!geometry.pointX.isFinite() || !geometry.pointY.isFinite()) {
      return EditorViewportAnchorScroll(currentScrollOffset, attachmentAchieved = false)
    }
    if (!maximumScrollOffset.x.isFinite() || !maximumScrollOffset.y.isFinite()) {
      return EditorViewportAnchorScroll(currentScrollOffset, attachmentAchieved = false)
    }
    if (maximumScrollOffset.x < 0f || maximumScrollOffset.y < 0f) {
      return EditorViewportAnchorScroll(currentScrollOffset, attachmentAchieved = false)
    }
    // Ordinary publication compensates only for document layout movement.
    // A pending zoom attachment still has an explicit viewport destination.
    val desiredScrollOffset =
      if (current.attachmentPending) {
        Offset(
          x = geometry.pointX - current.pointAttachmentX,
          y = geometry.pointY - current.pointAttachmentY,
        )
      } else {
        Offset(
          x = currentScrollOffset.x + geometry.pointX - current.geometry.pointX,
          y = currentScrollOffset.y + geometry.pointY - current.geometry.pointY,
        )
      }
    val scrollOffset =
      Offset(
        x = desiredScrollOffset.x.coerceIn(0f, maximumScrollOffset.x),
        y = desiredScrollOffset.y.coerceIn(0f, maximumScrollOffset.y),
      )
    return EditorViewportAnchorScroll(
      scrollOffset = scrollOffset,
      attachmentAchieved = scrollOffset == desiredScrollOffset,
    )
  }

  fun publicationRevealScroll(
    geometry: EditorViewportAnchorGeometry,
    currentScrollOffset: Offset,
    maximumScrollOffset: Offset,
    visibleArea: EditorVisibleArea,
    resolveReveal: ((EditorViewportAnchorRevealOrigin) -> Float?)? = null,
  ): EditorViewportAnchorScroll {
    val exact = publicationScroll(geometry, currentScrollOffset, maximumScrollOffset)
    if (!rectHeightChanged(active?.geometry?.rect, geometry.rect)) return exact
    active?.revealOrigin?.let { origin ->
      resolveReveal?.invoke(origin)?.let {
        return EditorViewportAnchorScroll(
          scrollOffset = exact.scrollOffset.copy(y = it.coerceIn(0f, maximumScrollOffset.y)),
          attachmentAchieved = true,
        )
      }
    }
    return EditorViewportAnchorScroll(
      scrollOffset =
        exact.scrollOffset.copy(
          y = resizeScroll(geometry, exact.scrollOffset.y, maximumScrollOffset.y, visibleArea)
        ),
      attachmentAchieved = true,
    )
  }

  fun acceptGeometry(
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
    attachmentAchieved: Boolean = true,
  ) {
    val current = active ?: return
    if (attachmentAchieved || !current.attachmentPending) {
      attachActive(
        identity = current.identity,
        geometry = geometry,
        scrollOffset = scrollOffset,
        revealOrigin = current.revealOrigin,
        source = current.source,
      )
    } else {
      active = current.copy(geometry = geometry)
    }
  }

  fun acceptGeometryAfterAutomaticScroll(
    geometry: EditorViewportAnchorGeometry,
    scrollOffset: Offset,
  ) {
    val current = active ?: return
    val attachmentAchieved =
      !current.attachmentPending ||
        (abs(scrollOffset.x - (geometry.pointX - current.pointAttachmentX)) <= 1f &&
          abs(scrollOffset.y - (geometry.pointY - current.pointAttachmentY)) <= 1f)
    acceptGeometry(geometry, scrollOffset, attachmentAchieved)
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
    // A viewport anchor tracks layout displacement; it is not a reveal request.
    if (active?.source != Source.Selection) return currentScrollY
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
