package co.typie.editor.external

import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.ExternalElementData
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImageResizePolicyTest {
  @Test
  fun proportion_scales_the_size_fitted_to_a_full_page() {
    val maxSize = imageResizeMaxSize(600f, 600f, 0.2f, 800f)
    assertEquals(Size(160f, 800f), maxSize)
    assertEquals(Size(80f, 400f), imageResizeSize(50f, maxSize))
    assertEquals(Size(16f, 80f), imageResizeSize(10f, maxSize))
    assertEquals(Size(600f, 3000f), imageResizeMaxSize(600f, 600f, 0.2f, null))
  }

  @Test
  fun very_narrow_images_can_shrink_to_ten_percent() {
    val maxSize = imageResizeMaxSize(600f, 600f, 0.03f, 800f)
    assertEquals(Size(24f, 800f), maxSize)
    val minimum = imageResizeSize(10f, maxSize)
    assertEquals(2.4f, minimum.width, 0.00001f)
    assertEquals(80f, minimum.height)
    assertEquals(minimum, imageResizeSize(0f, maxSize))
    assertEquals(maxSize, imageResizeSize(200f, maxSize))
  }

  @Test
  fun original_size_and_container_width_bound_the_reference_size() {
    val smallOriginal = imageResizeMaxSize(800f, 320f, 2f, null)
    val narrowContainer = imageResizeMaxSize(200f, 320f, 2f, null)
    assertEquals(Size(320f, 160f), smallOriginal)
    assertEquals(Size(160f, 80f), imageResizeSize(50f, smallOriginal))
    assertEquals(Size(200f, 100f), narrowContainer)
    assertEquals(Size(100f, 50f), imageResizeSize(50f, narrowContainer))
  }

  @Test
  fun resize_draft_keeps_its_reference_size_until_commit() {
    val state = EditorExternalImageElementState()
    state.assets["asset"] = EditorImageAsset("asset", "", "", 600, 3000, 0.2, null)
    val image = ExternalElementData.Image(id = "asset", proportion = 100, maxHeight = 800f)
    val maxSize = imageResizeMaxSize(600f, 600f, 0.2f, 800f)
    state.resizeDrafts["image"] = EditorImageResizeDraft(50f, maxSize)
    assertEquals(Size(80f, 400f), state.displaySize("image", image, 100f))
    state.clearResizeState("image")
    assertEquals(Size(100f, 500f), state.displaySize("image", image, 100f))
  }
}
