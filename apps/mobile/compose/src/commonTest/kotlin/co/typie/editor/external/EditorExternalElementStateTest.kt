package co.typie.editor.external

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorExternalElementStateTest {
  @Test
  fun putResolvedClearsAStaleMissingResolution() {
    val state = EditorExternalElementState()
    val asset =
      EditorImageAsset(
        id = "x",
        url = "https://cdn/x",
        width = 10,
        height = 20,
        ratio = 0.5,
        placeholder = null,
      )
    state.resolutions["x"] = EditorAssetResolution.Missing

    state.putResolved(asset)

    assertEquals(asset, state.images.assets["x"])
    assertNull(state.resolutions["x"])
  }

  @Test
  fun putResolvedClearsAStalePendingResolution() {
    val state = EditorExternalElementState()
    val asset = EditorFileAsset(id = "x", name = "x.txt", url = "https://cdn/x", size = 10)
    state.resolutions["x"] =
      EditorAssetResolution.Pending(
        EditorAssetPendingMeta(kind = "file", name = "x.txt", size = 10)
      )

    state.putResolved(asset)

    assertEquals(asset, state.files.assets["x"])
    assertNull(state.resolutions["x"])
  }

  @Test
  fun putResolvedIsANoOpOnResolutionsWhenNoneWasTracked() {
    val state = EditorExternalElementState()
    val asset =
      EditorImageAsset(
        id = "x",
        url = "https://cdn/x",
        width = 10,
        height = 20,
        ratio = 0.5,
        placeholder = null,
      )

    state.putResolved(asset)

    assertEquals(asset, state.images.assets["x"])
    assertNull(state.resolutions["x"])
  }
}
