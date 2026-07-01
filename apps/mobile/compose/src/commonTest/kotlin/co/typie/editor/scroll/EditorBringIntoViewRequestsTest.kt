package co.typie.editor.scroll

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorBringIntoViewRequestsTest {
  private val overlayTarget =
    EditorBringIntoViewTarget.OverlayRect(
      pageIdx = 0,
      left = 0f,
      top = 10f,
      width = 20f,
      height = 30f,
    )

  @Test
  fun `bring-into-view target attaches to requested editor version only`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 11L,
    )

    assertNull(requests.activateForVersion(version = 10L))
    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 11L),
    )
    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 11L),
    )

    assertTrue(
      requests.markApplied(
        version = 11L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )
    assertFalse(
      requests.markApplied(
        version = 11L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )
    assertNull(requests.activateForVersion(version = 11L))
  }

  @Test
  fun `new bring-into-view request does not cancel active target for current editor version`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 2L,
    )
    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 2L),
    )

    requests.requestForVersion(target = overlayTarget, version = 3L)

    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 2L),
    )
    assertTrue(
      requests.markApplied(
        version = 2L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )
    assertNull(requests.activateForVersion(version = 2L))
    assertEquals(request(overlayTarget), requests.activateForVersion(version = 3L))
  }

  @Test
  fun `newer request does not replace previous request before previous target version is built`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 259L,
    )
    assertNull(requests.activateForVersion(version = 258L))

    requests.requestForVersion(target = overlayTarget, version = 260L)

    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 259L),
    )
    assertTrue(
      requests.markApplied(
        version = 259L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )

    assertEquals(request(overlayTarget), requests.activateForVersion(version = 260L))
  }

  @Test
  fun `requests for consecutive editor versions attach to consecutive scroll frames`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 291L,
    )
    requests.requestForVersion(target = overlayTarget, version = 292L)

    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 291L),
    )
    assertTrue(
      requests.markApplied(
        version = 291L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )

    assertEquals(request(overlayTarget), requests.activateForVersion(version = 292L))
  }

  @Test
  fun `skipped editor versions collapse to latest eligible bring-into-view request`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 291L,
    )
    requests.requestForVersion(target = overlayTarget, version = 292L)

    assertEquals(request(overlayTarget), requests.activateForVersion(version = 292L))
  }

  @Test
  fun `cancel clears active and queued bring-into-view targets`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 1L,
    )
    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 1L),
    )

    requests.requestForVersion(target = overlayTarget, version = 2L)
    requests.cancel()

    assertFalse(
      requests.markApplied(
        version = 1L,
        request = request(EditorBringIntoViewTarget.CurrentSelectionHead),
      )
    )
    assertNull(requests.activateForVersion(version = 1L))
    assertNull(requests.activateForVersion(version = 2L))
  }

  @Test
  fun `activating same version does not consume active bring-into-view target`() {
    val requests = EditorBringIntoViewRequests()

    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 1L,
    )

    requests.activateForVersion(version = 1L)
    requests.activateForVersion(version = 1L)

    assertEquals(
      request(EditorBringIntoViewTarget.CurrentSelectionHead),
      requests.activateForVersion(version = 1L),
    )
  }

  private fun request(
    target: EditorBringIntoViewTarget,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  ): EditorBringIntoViewRequests.Request =
    EditorBringIntoViewRequests.Request(target = target, behavior = behavior)
}
