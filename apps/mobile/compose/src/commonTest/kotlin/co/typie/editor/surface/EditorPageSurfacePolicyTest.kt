package co.typie.editor.surface

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.FrameProof
import co.typie.editor.PresentedFrame
import co.typie.editor.SurfaceKey
import co.typie.editor.ffi.FrameKey
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class EditorPageSurfacePolicyTest {
  @Test
  fun `published frame keeps its committed pixel size across a density and zoom replacement`() {
    val frame =
      PresentedFrame(
        bitmap = ImageBitmap(101, 199),
        pixelSize = IntSize(101, 199),
        proof = FrameProof(editorRevision = 1, surfaceKey = SurfaceKey(1), frameKey = FrameKey(1)),
      )

    val display = resolveFrameDisplay(publishedFrame = frame, desiredPixelSize = IntSize(152, 301))

    assertSame(frame, display.frame)
    assertEquals(IntSize(101, 199), display.pixelSize)
  }

  @Test
  fun `current density and zoom target pixels are used before the first published frame`() {
    val display = resolveFrameDisplay(publishedFrame = null, desiredPixelSize = IntSize(152, 301))

    assertEquals(null, display.frame)
    assertEquals(IntSize(152, 301), display.pixelSize)
  }
}
