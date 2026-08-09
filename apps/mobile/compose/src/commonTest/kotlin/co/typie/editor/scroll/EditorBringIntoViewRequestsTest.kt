package co.typie.editor.scroll

import co.typie.editor.EditorState
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Rect as FfiRect
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class EditorBringIntoViewRequestsTest {
  private val pageRectsTarget =
    EditorBringIntoViewTarget.PageRects(
      listOf(PageRect(pageIdx = 0, rect = FfiRect(x = 0f, y = 10f, width = 20f, height = 30f)))
    )

  @Test
  fun `state-based request derives target and version from the same state`() {
    val requests = EditorBringIntoViewRequests()
    val state = EditorState.Initial.copy(version = 11L, selectionHitRects = pageRectsTarget.rects)

    assertTrue(
      requests.requestForState(
        state,
        policy = EditorBringIntoViewPolicy.Reveal,
        behavior = EditorBringIntoViewBehavior.Smooth,
      ) {
        selectionHitRects.toPageRectsTarget()
      }
    )

    assertNull(requests.activateForVersion(version = 10L))
    val request = requests.activateForVersion(version = 11L)
    assertTrue(request?.target == pageRectsTarget)
    assertTrue(request?.policy == EditorBringIntoViewPolicy.Reveal)
    assertTrue(request?.behavior == EditorBringIntoViewBehavior.Smooth)
  }

  @Test
  fun `request behavior defaults to instant independently from reveal policy`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = pageRectsTarget,
      version = 11L,
      policy = EditorBringIntoViewPolicy.Reveal,
    )

    val request = requests.activateForVersion(version = 11L)
    assertTrue(request?.behavior == EditorBringIntoViewBehavior.Instant)
  }

  @Test
  fun `request accepts smooth behavior independently from reveal policy`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = pageRectsTarget,
      version = 11L,
      policy = EditorBringIntoViewPolicy.Reveal,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )

    val request = requests.activateForVersion(version = 11L)
    assertTrue(request?.behavior == EditorBringIntoViewBehavior.Smooth)
  }

  @Test
  fun `standalone declaration wakes Host presentation reconciliation`() {
    var wakes = 0
    val requests = EditorBringIntoViewRequests(requestPresentation = { wakes += 1 })

    requests.requestForVersion(
      EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 1L,
      policy = EditorBringIntoViewPolicy.CursorGuard,
    )

    assertTrue(wakes > 0)
  }

  @Test
  fun `state-based request reports when state has no target`() {
    val requests = EditorBringIntoViewRequests()

    assertFalse(
      requests.requestForState(
        EditorState.Initial,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      ) {
        null
      }
    )
    assertNull(requests.activateForVersion(version = EditorState.Initial.version))
  }

  @Test
  fun `semantic request remains eligible for a later satisfying revision`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 11L,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )

    assertNull(requests.activateForVersion(version = 10L))
    assertSame(request, requests.activateForVersion(version = 11L))
    assertSame(request, requests.activateForVersion(version = 12L))
  }

  @Test
  fun `new declaration immediately supersedes the previous reveal`() {
    val requests = EditorBringIntoViewRequests()
    val previous =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        1L,
        EditorBringIntoViewPolicy.CursorGuard,
      )
    val current = requests.requestForVersion(pageRectsTarget, 2L, EditorBringIntoViewPolicy.Reveal)

    assertTrue(previous.presentation.isCompleted)
    assertSame(current, requests.activateForVersion(version = 2L))
  }

  @Test
  fun `late binding cannot replace a newer declaration`() {
    val requests = EditorBringIntoViewRequests()
    val previous =
      requests.declare(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        EditorBringIntoViewPolicy.CursorGuard,
      )
    val current = requests.declare(pageRectsTarget, EditorBringIntoViewPolicy.Reveal)

    assertFalse(requests.bind(previous, version = 1L))
    assertTrue(requests.bind(current, version = 2L))
    assertSame(current, requests.activateForVersion(version = 2L))
  }

  @Test
  fun `exact page rect request becomes obsolete when its revision is skipped`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        pageRectsTarget,
        version = 2L,
        policy = EditorBringIntoViewPolicy.Reveal,
      )

    assertNull(requests.activateForVersion(version = 3L))
    assertFalse(request.presentation.isCompleted)
    requests.discardObsoleteForVersion(version = 3L)
    assertTrue(request.presentation.isCompleted)
  }

  @Test
  fun `pointer selection request is exact and becomes obsolete when its revision is skipped`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 2L,
        policy = EditorBringIntoViewPolicy.PointerCursorGuard,
      )

    assertNull(requests.activateForVersion(version = 1L))
    assertSame(request, requests.activateForVersion(version = 2L))
    assertNull(requests.activateForVersion(version = 3L))
    requests.discardObsoleteForVersion(version = 3L)
    assertTrue(request.presentation.isCompleted)
  }

  @Test
  fun `new request supersedes an exact pointer selection reveal`() {
    val requests = EditorBringIntoViewRequests()
    val pointer =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 2L,
        policy = EditorBringIntoViewPolicy.PointerCursorGuard,
      )
    val current =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 3L,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )

    assertTrue(pointer.presentation.isCompleted)
    assertSame(current, requests.activateForVersion(version = 3L))
  }

  @Test
  fun `matching presentation completes and clears the current reveal`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        1L,
        EditorBringIntoViewPolicy.CursorGuard,
      )

    assertTrue(requests.markPresented(version = 1L, request = request))
    assertTrue(request.presentation.isCompleted)
    assertNull(requests.activateForVersion(version = 1L))
    assertFalse(requests.markPresented(version = 1L, request = request))
  }

  @Test
  fun `surface failure discards only a reveal eligible for the failed version`() {
    val requests = EditorBringIntoViewRequests()
    val previous =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 1L,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )

    requests.discardFailedForVersion(version = 1L)

    assertTrue(previous.presentation.isCompleted)

    val current =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 3L,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )
    requests.discardFailedForVersion(version = 2L)

    assertFalse(current.presentation.isCompleted)
    assertSame(current, requests.activateForVersion(version = 3L))
  }

  @Test
  fun `cancel completes and clears the current reveal`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        1L,
        EditorBringIntoViewPolicy.CursorGuard,
      )

    requests.cancel()

    assertTrue(request.presentation.isCompleted)
    assertNull(requests.activateForVersion(version = 1L))
  }
}
