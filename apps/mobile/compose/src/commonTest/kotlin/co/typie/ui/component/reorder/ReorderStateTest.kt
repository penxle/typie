package co.typie.ui.component.reorder

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReorderStateTest {
  @Test
  fun `layout order stays at drag start until a moved release`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("c"))
    assertTrue(state.moveDraggedTo(0))

    assertEquals(listOf("c", "a", "b"), state.keys)
    assertEquals(listOf("a", "b", "c"), state.layoutKeys)

    assertEquals(0, state.endDrag(releaseOffsetY = 0f)?.toIndex)
    assertEquals(listOf("c", "a", "b"), state.layoutKeys)
  }

  @Test
  fun `moving a drag to a different target returns one drop`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("a"))
    assertTrue(state.moveDraggedTo(2))

    assertEquals(listOf("b", "c", "a"), state.keys)
    assertEquals(
      ReorderDrop(movedKey = "a", fromIndex = 0, toIndex = 2, orderedKeys = listOf("b", "c", "a")),
      state.endDrag(releaseOffsetY = 15f),
    )
    assertNull(state.draggingKey)
    assertEquals(15f, state.settlingOffsetY("a"))
  }

  @Test
  fun `same target release produces no drop but preserves settling offset`() {
    val state = ReorderState(listOf("a", "b"))

    assertTrue(state.beginDrag("a"))

    assertNull(state.endDrag(releaseOffsetY = 12f))
    assertEquals(listOf("a", "b"), state.keys)
    assertEquals(12f, state.settlingOffsetY("a"))

    state.clearSettling("a")
    assertNull(state.settlingOffsetY("a"))
  }

  @Test
  fun `cancel restores latest input when no local drop was pending`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("a"))
    assertTrue(state.moveDraggedTo(2))
    state.inputKeys = listOf("c", "b", "a")
    state.cancelDrag()

    assertEquals(listOf("c", "b", "a"), state.keys)
    assertNull(state.draggingKey)
  }

  @Test
  fun `cancel of a later drag restores the prior local dropped order`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("a"))
    assertTrue(state.moveDraggedTo(2))
    assertEquals(2, state.endDrag(releaseOffsetY = 0f)?.toIndex)
    assertEquals(listOf("b", "c", "a"), state.keys)

    assertTrue(state.beginDrag("b"))
    assertTrue(state.moveDraggedTo(2))
    state.cancelDrag()

    assertEquals(listOf("b", "c", "a"), state.keys)
  }

  @Test
  fun `input update does not replace a pending local dropped order`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("a"))
    assertTrue(state.moveDraggedTo(2))
    state.endDrag(releaseOffsetY = 0f)
    state.inputKeys = listOf("a", "c", "b")

    assertEquals(listOf("b", "c", "a"), state.keys)

    state.cancelDrag()
    assertEquals(listOf("a", "c", "b"), state.keys)
  }

  @Test
  fun `membership change replaces a pending local dropped order`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("a"))
    assertTrue(state.moveDraggedTo(2))
    state.endDrag(releaseOffsetY = 0f)
    state.inputKeys = listOf("a", "b", "c", "d")

    assertEquals(listOf("a", "b", "c", "d"), state.keys)
  }

  @Test
  fun `removing the dragged key cancels without producing a drop`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("b"))
    assertTrue(state.moveDraggedTo(0))
    state.inputKeys = listOf("a", "c")

    assertNull(state.draggingKey)
    assertEquals(listOf("a", "c"), state.keys)
    assertNull(state.endDrag(releaseOffsetY = 0f))
  }

  @Test
  fun `membership change cancels the active drag and adopts the latest input`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("b"))
    assertTrue(state.moveDraggedTo(2))
    state.inputKeys = listOf("a", "b", "c", "d")

    assertFalse(state.isDragging)
    assertEquals(listOf("a", "b", "c", "d"), state.keys)
  }

  @Test
  fun `moving to the current target is a no-op`() {
    val state = ReorderState(listOf("a", "b", "c"))

    assertTrue(state.beginDrag("b"))
    val initialRevision = state.orderRevision

    assertFalse(state.moveDraggedTo(1))
    assertEquals(initialRevision, state.orderRevision)
  }
}
