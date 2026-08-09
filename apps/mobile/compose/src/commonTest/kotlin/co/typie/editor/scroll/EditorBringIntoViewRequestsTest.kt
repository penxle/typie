package co.typie.editor.scroll

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class EditorBringIntoViewRequestsTest {
  private val trackedItemTarget = EditorBringIntoViewTarget.TrackedItem("comment-1")

  @Test
  fun `request behavior defaults to instant independently from reveal policy`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = trackedItemTarget,
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
      target = trackedItemTarget,
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
  fun `tracked item request remains eligible when a later revision provides its geometry`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        target = EditorBringIntoViewTarget.TrackedItem("comment-1"),
        version = 11L,
        policy = EditorBringIntoViewPolicy.Reveal,
      )

    assertSame(request, requests.activateForVersion(version = 12L))
    assertFalse(request.presentation.isCompleted)
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
    val current =
      requests.requestForVersion(trackedItemTarget, 2L, EditorBringIntoViewPolicy.Reveal)

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
    val current = requests.declare(trackedItemTarget, EditorBringIntoViewPolicy.Reveal)

    assertFalse(requests.bind(previous, version = 1L))
    assertTrue(requests.bind(current, version = 2L))
    assertSame(current, requests.activateForVersion(version = 2L))
  }

  @Test
  fun `pointer selection request remains eligible when a later revision provides geometry`() {
    val requests = EditorBringIntoViewRequests()
    val request =
      requests.requestForVersion(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        version = 2L,
        policy = EditorBringIntoViewPolicy.PointerCursorGuard,
      )

    assertNull(requests.activateForVersion(version = 1L))
    assertSame(request, requests.activateForVersion(version = 2L))
    assertSame(request, requests.activateForVersion(version = 3L))
    assertFalse(request.presentation.isCompleted)
  }

  @Test
  fun `new request supersedes a pending pointer selection reveal`() {
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
