package co.typie.editor

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.FrameKey
import kotlin.jvm.JvmInline

@JvmInline internal value class SurfaceKey(val value: Long)

internal data class SurfaceTarget(
  val page: Int,
  val key: SurfaceKey,
  val configuration: SurfaceConfiguration,
)

internal data class FrameProof(
  val editorRevision: Long,
  val surfaceKey: SurfaceKey,
  val frameKey: FrameKey,
)

internal data class PresentedFrame(
  val bitmap: ImageBitmap,
  val pixelSize: IntSize,
  val proof: FrameProof,
)

/**
 * Pure publication rules. Mutable editor, target, waiter, and surface-operation facts stay in
 * [Editor]; this object only answers questions about those facts.
 */
internal object Publication {
  fun matchesTargets(
    frames: Map<Int, PresentedFrame>,
    targets: Collection<SurfaceTarget>,
  ): Boolean =
    frames.size == targets.size &&
      targets.all { target -> frames[target.page]?.proof?.surfaceKey == target.key }

  fun accepts(
    proof: FrameProof,
    target: SurfaceTarget,
    requiredRevision: Long?,
    available: Boolean,
  ): Boolean =
    available &&
      proof.surfaceKey == target.key &&
      (requiredRevision == null || proof.editorRevision >= requiredRevision)

  fun preparingPage(
    hasVisualHost: Boolean,
    hasPublishedFrames: Boolean,
    appliedRevision: Long,
    publishedRevision: Long?,
    appliedPageCount: Int,
    targetCount: Int,
  ): Int? =
    if (
      hasVisualHost &&
        hasPublishedFrames &&
        publishedRevision != null &&
        appliedRevision > publishedRevision &&
        appliedPageCount > 0 &&
        targetCount == 0
    ) {
      0
    } else {
      null
    }

  fun canPublish(
    hasVisualHost: Boolean,
    hasPublishedFrames: Boolean,
    appliedRevision: Long,
    publishedRevision: Long,
    pages: Collection<PageFacts>,
    targetsChanged: Boolean = false,
  ): Boolean =
    hasVisualHost &&
      (!hasPublishedFrames || pages.isNotEmpty()) &&
      appliedRevision >= publishedRevision &&
      (appliedRevision > publishedRevision ||
        pages.any { it.requiredRevision != null } ||
        targetsChanged) &&
      pages.all { page ->
        page.requiredRevision == null ||
          page.proof?.let {
            accepts(
              proof = it,
              target = page.target,
              requiredRevision = page.requiredRevision,
              available = page.available,
            )
          } == true
      }

  fun workRevision(appliedRevision: Long, requiredRevision: Long?): Long? = requiredRevision?.let {
    maxOf(appliedRevision, it)
  }

  internal data class PageFacts(
    val target: SurfaceTarget,
    val requiredRevision: Long?,
    val proof: FrameProof?,
    val available: Boolean,
  )
}
