package co.typie.screen.editor.editor.overlay

import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.external.EditorExternalImageElementState
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorContextMenuImageTest {
  private val images =
    EditorExternalImageElementState().apply {
      assets["asset"] = EditorImageAsset("asset", "preview", "original", 200, 100, 2.0, null)
    }
  private val image =
    ExternalElement(
      pageIdx = 0,
      node = "image",
      bounds = Rect(20f, 40f, 400f, 100f),
      isSelected = true,
      data = ExternalElementData.Image(id = "asset", proportion = 50),
    )
  private val selection =
    Selection(Position("root", 0, Affinity.Downstream), Position("root", 1, Affinity.Downstream))
  private val state =
    EditorState.Initial.copy(selection = selection, externalElements = listOf(image))

  @Test
  fun touchUsesTheSingleSelectedImageIncludingParentSlotPositions() {
    assertEquals(image, state.contextMenuImage(null, images))
    assertNull(
      state
        .copy(selection = selection.copy(head = selection.head.copy(offset = 2)))
        .contextMenuImage(null, images)
    )
    assertNull(
      state
        .copy(externalElements = listOf(image.copy(isSelected = false)))
        .contextMenuImage(null, images)
    )
  }

  @Test
  fun pointerTargetsTheImageEvenWithinAWiderSelection() {
    val range = selection.copy(head = selection.head.copy(offset = 4))
    assertEquals(
      image,
      state.copy(selection = range).contextMenuImage(PagePoint(0, 220f, 60f), images),
    )
  }

  @Test
  fun pointerMustHitTheRenderedImageRatherThanItsFullWidthBlockOrAnotherPage() {
    assertNull(state.contextMenuImage(PagePoint(0, 30f, 60f), images))
    assertNull(state.contextMenuImage(PagePoint(0, 220f, 130f), images))
    assertNull(state.contextMenuImage(PagePoint(1, 220f, 60f), images))
    assertEquals(image, state.contextMenuImage(PagePoint(0, 220f, 60f), images))
  }
}
