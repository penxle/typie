package co.typie.editor

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.FrameKey
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PublicationTest {
  private val configuration = SurfaceConfiguration(width = 100.0, height = 200.0, scaleFactor = 2.0)

  @Test
  fun proofMustMatchCurrentSurfaceAndRequiredRevision() {
    val target = SurfaceTarget(page = 0, key = SurfaceKey(2), configuration = configuration)

    assertFalse(
      Publication.accepts(
        proof = FrameProof(12, SurfaceKey(1), FrameKey(10)),
        target = target,
        requiredRevision = 10,
        available = true,
      )
    )
    assertFalse(
      Publication.accepts(
        proof = FrameProof(9, SurfaceKey(2), FrameKey(11)),
        target = target,
        requiredRevision = 10,
        available = true,
      )
    )
    assertTrue(
      Publication.accepts(
        proof = FrameProof(11, SurfaceKey(2), FrameKey(12)),
        target = target,
        requiredRevision = 10,
        available = true,
      )
    )
  }

  @Test
  fun acceptedPublicationRemainsValidWhenTheProducerCohortChanges() {
    assertTrue(
      Publication.satisfiesWaiter(
        requestedRevision = 10,
        publishedRevision = 10,
        frames = mapOf(0 to frame(surfaceKey = 2)),
      )
    )
  }

  @Test
  fun emptyPublicationSatisfiesAWaiterWhenNoSurfaceIsRequired() {
    assertTrue(
      Publication.satisfiesWaiter(
        requestedRevision = 10,
        publishedRevision = 10,
        frames = emptyMap(),
      )
    )
    assertFalse(
      Publication.satisfiesWaiter(
        requestedRevision = 10,
        publishedRevision = 10,
        frames = emptyMap(),
        requireFrame = true,
      )
    )
  }

  @Test
  fun inheritedRequirementCapturesLatestAppliedRevision() {
    assertTrue(Publication.workRevision(appliedRevision = 11, requiredRevision = 10) == 11L)
    assertNull(Publication.workRevision(appliedRevision = 11, requiredRevision = null))
  }

  private fun frame(surfaceKey: Long): PresentedFrame =
    PresentedFrame(
      bitmap = ImageBitmap(width = 1, height = 1),
      pixelSize = IntSize(width = 1, height = 1),
      proof =
        FrameProof(editorRevision = 10, surfaceKey = SurfaceKey(surfaceKey), frameKey = FrameKey(1)),
    )
}
