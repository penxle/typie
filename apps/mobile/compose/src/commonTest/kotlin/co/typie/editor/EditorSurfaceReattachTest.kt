package co.typie.editor

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

class EditorSurfaceReattachTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun unattachedAppliedPageClassifiesSurfaceDeliveryFailureWithoutThrowing() =
    runTest(dispatcher) {
      val failure = IllegalStateException("regrown surface attach failed")
      val reported = mutableListOf<Throwable>()
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) })
      val editor = Editor(fake, this, dispatcher, onError = { _, error -> reported += error })
      fake.applySnapshot(editor)
      editor.activateVisualHost(Any())
      editor.requestSurfacePages(setOf(0))

      val escaped =
        runCatching { editor.surfaceDeliveryFailed(page = 0, session = null, error = failure) }
          .exceptionOrNull()
      advanceUntilIdle()

      assertNull(escaped)
      assertTrue(editor.terminal)
      assertEquals(listOf<Throwable>(failure), reported)
    }

  @Test
  fun regrownPageReattachesItsSurvivingBufferWithAFreshSessionAndProof() =
    runTest(dispatcher) {
      var pageExists = true
      val fake =
        FakeFfiEditor(
          pageSizesProvider = {
            if (pageExists) listOf(Size(width = 100f, height = 100f)) else emptyList()
          }
        )
      val editor = Editor(fake, this, dispatcher)
      fake.applySnapshot(editor)
      val host = Any()
      editor.activateVisualHost(host)

      try {
        var firstFrameKey: FrameKey? = null
        val firstSession = editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { firstFrameKey = it }
        editor.requestSurfacePages(setOf(0))
        advanceUntilIdle()
        val firstBitmap =
          editor.deliverFrame(
            firstSession,
            editor.appliedRevision,
            requireNotNull(firstFrameKey).value,
          )
        advanceUntilIdle()
        requireNotNull(editor.publishIfReady(setOf(0))).let { bundle ->
          assertTrue(editor.acceptPublication(bundle))
          editor.completePresentation(bundle)
        }
        val firstBundle = requireNotNull(editor.publishedBundle)
        val firstProof = requireNotNull(firstBundle.frames[0]?.proof)

        pageExists = false
        fake.applySnapshot(editor)
        advanceUntilIdle()

        assertTrue(firstSession.isRetired)
        assertSame(firstBundle, editor.publishedBundle)
        assertSame(firstBitmap, editor.publishedBundle?.frames?.get(0)?.bitmap)
        assertEquals(1, fake.attachCount)
        assertEquals(1, fake.detachCount)

        pageExists = true
        fake.applySnapshot(editor)
        var replacementFrameKey: FrameKey? = null
        val replacementSession =
          editor.attachSurface(0, 10L, 100.0, 100.0, 1.0) { replacementFrameKey = it }
        advanceUntilIdle()

        assertNotSame(firstSession, replacementSession)
        assertFalse(replacementSession.isRetired)
        assertEquals(2, fake.attachCount)
        assertEquals(
          listOf("attach:0:10", "detach:0", "attach:0:10"),
          fake.surfaceEvents.filter { it.startsWith("attach:") || it.startsWith("detach:") },
        )
        assertSame(firstBundle, editor.publishedBundle)

        firstSession.detach()
        advanceUntilIdle()

        assertEquals(2, fake.attachCount)
        assertEquals(1, fake.detachCount)

        val regrownRevision = editor.appliedRevision
        val replacementBitmap =
          editor.deliverFrame(
            replacementSession,
            regrownRevision,
            requireNotNull(replacementFrameKey).value,
          )
        advanceUntilIdle()
        requireNotNull(editor.publishIfReady(setOf(0))).let { bundle ->
          assertTrue(editor.acceptPublication(bundle))
          editor.completePresentation(bundle)
        }

        val replacementProof = requireNotNull(editor.publishedBundle?.frames?.get(0)?.proof)
        assertEquals(regrownRevision, editor.publishedRevision)
        assertEquals(regrownRevision, replacementProof.editorRevision)
        assertNotEquals(firstProof.surfaceKey, replacementProof.surfaceKey)
        assertSame(replacementBitmap, editor.publishedBundle?.frames?.get(0)?.bitmap)
        assertEquals(2, fake.attachCount)
      } finally {
        editor.deactivateVisualHost(host)
      }
    }
}

private val FakeFfiEditor.attachCount: Int
  get() = surfaceEvents.count { it.startsWith("attach:") }

private val FakeFfiEditor.detachCount: Int
  get() = surfaceEvents.count { it.startsWith("detach:") }

private fun Editor.deliverFrame(
  session: SurfaceSessionHandle,
  editorRevision: Long,
  frameKey: Long,
): ImageBitmap {
  val bitmap = ImageBitmap(width = 100, height = 100)
  deliverFrame(
    session = session,
    bitmap = bitmap,
    pixelSize = IntSize(width = 100, height = 100),
    editorRevision = editorRevision,
    frameKey = frameKey,
  )
  return bitmap
}
