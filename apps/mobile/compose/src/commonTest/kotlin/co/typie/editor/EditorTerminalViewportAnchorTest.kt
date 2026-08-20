package co.typie.editor

import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.ffi.ViewportAnchorResolution
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

class EditorTerminalViewportAnchorTest {
  private val point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 0f)
  private val anchor = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)

  @Test
  fun failed_editor_returns_unavailable_viewport_anchor_reads() = runTest {
    var selectionCaptureCount = 0
    var pointCaptureCount = 0
    var resolutionCount = 0
    val editor =
      Editor(
        FakeFfiEditor(
          captureSelectionViewportAnchorProvider = {
            selectionCaptureCount += 1
            error("terminal editor must not capture the selection anchor")
          },
          captureViewportAnchorAtProvider = { _, _ ->
            pointCaptureCount += 1
            error("terminal editor must not capture a point anchor")
          },
          resolveViewportAnchorProvider = { _, _ ->
            resolutionCount += 1
            error("terminal editor must not resolve an anchor")
          },
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    editor.fail(RuntimeException("boom"))

    assertNull(editor.captureSelectionViewportAnchor(revision = 1L))
    assertNull(
      editor.captureViewportAnchorAt(
        revision = 1L,
        point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 0f),
      )
    )
    assertEquals(
      ViewportAnchorResolution.Unavailable,
      editor.resolveViewportAnchor(revision = 1L, anchor = anchor),
    )
    assertEquals(0, selectionCaptureCount)
    assertEquals(0, pointCaptureCount)
    assertEquals(0, resolutionCount)
  }

  @Test
  fun selection_anchor_capture_contains_native_failure() = runTest {
    val failure = IllegalStateException("selection capture failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(captureSelectionViewportAnchorProvider = { throw failure }),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )

    assertNull(editor.captureSelectionViewportAnchor(revision = 1L))
    advanceUntilIdle()

    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
  }

  @Test
  fun point_anchor_capture_contains_native_failure() = runTest {
    val failure = IllegalStateException("point capture failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(captureViewportAnchorAtProvider = { _, _ -> throw failure }),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )

    assertNull(editor.captureViewportAnchorAt(revision = 1L, point = point))
    advanceUntilIdle()

    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
  }

  @Test
  fun anchor_resolution_contains_native_failure() = runTest {
    val failure = IllegalStateException("anchor resolution failed")
    val reported = mutableListOf<Throwable>()
    val editor =
      Editor(
        inner = FakeFfiEditor(resolveViewportAnchorProvider = { _, _ -> throw failure }),
        scope = this,
        dispatcher = StandardTestDispatcher(testScheduler),
        onError = { _, error -> reported += error },
      )

    assertEquals(
      ViewportAnchorResolution.Unavailable,
      editor.resolveViewportAnchor(revision = 1L, anchor = anchor),
    )
    advanceUntilIdle()

    assertTrue(editor.terminal)
    assertSame(failure, reported.single())
  }
}
