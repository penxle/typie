package co.typie.editor

import co.typie.editor.ffi.FrameKey
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PublicationTest {
  private val configuration = SurfaceConfiguration(width = 100.0, height = 200.0, scaleFactor = 2.0)

  @Test
  fun proofMustMatchCurrentKeyAndRequiredRevision() {
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
  fun bootstrapHostWithNoTargetsCanPublishButNoHostCannot() {
    assertTrue(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = false,
        appliedRevision = 2,
        publishedRevision = 1,
        pages = emptyList(),
      )
    )
    assertFalse(
      Publication.canPublish(
        hasVisualHost = false,
        hasPublishedFrames = false,
        appliedRevision = 2,
        publishedRevision = 1,
        pages = emptyList(),
      )
    )
  }

  @Test
  fun hostWithAPublishedFrameCannotPublishAnEmptyTargetSet() {
    assertFalse(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = true,
        appliedRevision = 2,
        publishedRevision = 1,
        pages = emptyList(),
      )
    )
  }

  @Test
  fun nonEmptyTargetShrinkCanPublishWhenTheRemainingProofIsReady() {
    val remaining = SurfaceTarget(page = 0, key = SurfaceKey(1), configuration = configuration)

    assertTrue(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = true,
        appliedRevision = 10,
        publishedRevision = 10,
        pages =
          listOf(
            Publication.PageFacts(
              target = remaining,
              requiredRevision = null,
              proof = FrameProof(10, remaining.key, FrameKey(1)),
              available = true,
            )
          ),
        targetsChanged = true,
      )
    )
  }

  @Test
  fun targetReplacementCanRepublishTheSameEditorRevision() {
    val target = SurfaceTarget(page = 0, key = SurfaceKey(2), configuration = configuration)
    val facts =
      Publication.PageFacts(
        target = target,
        requiredRevision = 10,
        proof = FrameProof(10, target.key, FrameKey(2)),
        available = true,
      )

    assertTrue(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = true,
        appliedRevision = 10,
        publishedRevision = 10,
        pages = listOf(facts),
      )
    )
  }

  @Test
  fun everyRequiredPageNeedsACurrentProof() {
    val first = SurfaceTarget(page = 0, key = SurfaceKey(1), configuration = configuration)
    val second = SurfaceTarget(page = 1, key = SurfaceKey(2), configuration = configuration)
    val firstProof = FrameProof(10, first.key, FrameKey(1))

    assertFalse(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = true,
        appliedRevision = 11,
        publishedRevision = 9,
        pages =
          listOf(
            Publication.PageFacts(first, 10, firstProof, available = true),
            Publication.PageFacts(second, 10, proof = null, available = true),
          ),
      )
    )
    assertTrue(
      Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = true,
        appliedRevision = 11,
        publishedRevision = 9,
        pages =
          listOf(
            Publication.PageFacts(first, 10, firstProof, available = true),
            Publication.PageFacts(
              second,
              10,
              FrameProof(10, second.key, FrameKey(2)),
              available = true,
            ),
          ),
      )
    )
  }

  @Test
  fun unavailableTargetCannotAcceptAFrameProof() {
    val target = SurfaceTarget(page = 0, key = SurfaceKey(1), configuration = configuration)

    assertFalse(
      Publication.accepts(
        proof = FrameProof(10, target.key, FrameKey(1)),
        target = target,
        requiredRevision = 10,
        available = false,
      )
    )
  }

  @Test
  fun inheritedRequirementCapturesLatestAppliedRevision() {
    assertTrue(Publication.workRevision(appliedRevision = 11, requiredRevision = 10) == 11L)
    assertNull(Publication.workRevision(appliedRevision = 11, requiredRevision = null))
  }
}
