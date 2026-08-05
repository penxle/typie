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

internal object Publication {
  fun satisfiesWaiter(
    requestedRevision: Long,
    publishedRevision: Long?,
    frames: Map<Int, PresentedFrame>,
    requireFrame: Boolean = false,
  ): Boolean =
    publishedRevision != null &&
      publishedRevision >= requestedRevision &&
      (!requireFrame || frames.isNotEmpty())

  fun accepts(
    proof: FrameProof,
    target: SurfaceTarget,
    requiredRevision: Long?,
    available: Boolean,
  ): Boolean =
    available &&
      proof.surfaceKey == target.key &&
      (requiredRevision == null || proof.editorRevision >= requiredRevision)

  fun workRevision(appliedRevision: Long, requiredRevision: Long?): Long? = requiredRevision?.let {
    maxOf(appliedRevision, it)
  }
}
