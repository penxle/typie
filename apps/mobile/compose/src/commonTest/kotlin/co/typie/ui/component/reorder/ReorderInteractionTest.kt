package co.typie.ui.component.reorder

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReorderInteractionTest {
  @Test
  fun `successive edge scroll layouts advance target while pointer stays fixed`() {
    val interaction = interaction(listOf("a", "b", "c", "d"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)

    val firstProposal = interaction.updatePointer(pointerY = 130f)
    assertEquals(1, firstProposal?.targetIndex)
    assertTrue(interaction.commitTarget(requireNotNull(firstProposal)))
    assertEquals(listOf("b", "a", "c", "d"), interaction.orderState.keys)

    val secondProposal =
      interaction.publishLayout(layoutSnapshot(item("b", 0, 0f, 50f), item("c", 2, 100f, 150f)))
    assertEquals(2, secondProposal?.targetIndex)
    assertTrue(interaction.commitTarget(requireNotNull(secondProposal)))
    assertEquals(listOf("b", "c", "a", "d"), interaction.orderState.keys)
  }

  @Test
  fun `stale layout after order revision is ignored until matching layout arrives`() {
    val interaction = interaction(listOf("a", "b", "c"))
    val initial =
      layoutSnapshot(item("a", 1, 0f, 50f), item("b", 2, 50f, 100f), item("c", 3, 100f, 150f))
    interaction.publishLayout(initial)
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)
    val proposal = requireNotNull(interaction.updatePointer(pointerY = 130f))
    assertTrue(interaction.commitTarget(proposal))

    assertNull(interaction.publishLayout(initial))
    assertNull(interaction.updatePointer(pointerY = 131f))

    val matching =
      layoutSnapshot(item("b", 1, 0f, 50f), item("c", 2, 50f, 100f), item("a", 3, 100f, 150f))
    interaction.publishLayout(matching)
    assertTrue(interaction.orderState.isDragging)
  }

  @Test
  fun `drag stays active when pinned item is absent from visible layout`() {
    val interaction = interaction(listOf("a", "b", "c"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)

    interaction.publishLayout(layoutSnapshot(item("b", 1, 0f, 50f), item("c", 2, 50f, 100f)))

    assertTrue(interaction.orderState.isDragging)
    assertEquals("a", interaction.orderState.draggingKey)
  }

  @Test
  fun `stale pinned item index does not block a matching scrolled layout`() {
    val interaction = interaction(listOf("a", "b", "c", "d"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)
    val firstProposal = requireNotNull(interaction.updatePointer(pointerY = 130f))
    assertTrue(interaction.commitTarget(firstProposal))

    val proposal =
      interaction.publishLayout(
        layoutSnapshot(item("a", 0, -100f, -50f), item("c", 2, 50f, 100f), item("d", 3, 100f, 150f))
      )

    assertEquals(3, proposal?.targetIndex)
  }

  @Test
  fun `stale animated sibling does not hide matching visible slots`() {
    val interaction = interaction(listOf("a", "b", "c", "d"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)
    val firstProposal = requireNotNull(interaction.updatePointer(pointerY = 130f))
    assertTrue(interaction.commitTarget(firstProposal))

    val proposal =
      interaction.publishLayout(
        layoutSnapshot(item("b", 1, -50f, 0f), item("c", 2, 50f, 100f), item("d", 3, 100f, 150f))
      )

    assertEquals(3, proposal?.targetIndex)
  }

  @Test
  fun `edge scroll direction overrides pointer jitter after reversing`() {
    val keys = (0..70).map(Int::toString)
    val interaction = interaction(keys)
    interaction.publishLayout(layoutSnapshot(item("60", 60, 0f, 10f), item("61", 61, 10f, 20f)))
    assertTrue(interaction.beginDrag("60", pointerY = 5f, pointerOffsetInItemY = 5f))
    interaction.updateDraggedSize(height = 10f)
    val downwardProposal = requireNotNull(interaction.updatePointer(pointerY = 26f))
    assertTrue(interaction.commitTarget(downwardProposal))
    assertEquals(61, interaction.orderState.keys.indexOf("60"))

    interaction.updatePointer(pointerY = 5f)
    interaction.updatePointer(pointerY = 6f)
    val upwardProposal =
      interaction.publishLayout(
        layoutSnapshot(
          *(20..30)
            .map { index -> item(index.toString(), index, (index - 20) * 10f, (index - 19) * 10f) }
            .toTypedArray()
        ),
        scrollDirection = -1,
      )

    val upwardTarget = requireNotNull(upwardProposal).targetIndex
    assertTrue(upwardTarget < 61)
    assertTrue(upwardTarget in 20..30)
  }

  @Test
  fun `pointer cancellation produces no drop`() {
    val interaction = activeInteraction()

    interaction.cancel()

    assertFalse(interaction.orderState.isDragging)
    assertNull(interaction.release())
  }

  @Test
  fun `removing dragged key cancels without a drop`() {
    val interaction = interaction(listOf("a", "b", "c"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("b", pointerY = 75f, pointerOffsetInItemY = 25f))

    interaction.updateInputKeys(listOf("a", "c"))

    assertFalse(interaction.orderState.isDragging)
    assertNull(interaction.release())
  }

  private fun activeInteraction(): ReorderInteraction<String> {
    val interaction = interaction(listOf("a", "b"))
    interaction.publishLayout(layoutSnapshot(item("a", 0, 0f, 50f), item("b", 1, 50f, 100f)))
    assertTrue(interaction.beginDrag("a", pointerY = 25f, pointerOffsetInItemY = 25f))
    interaction.updateDraggedSize(height = 50f)
    return interaction
  }

  private fun interaction(keys: List<String>) = ReorderInteraction(ReorderState(keys))

  private fun item(key: String, lazyIndex: Int, top: Float, bottom: Float) =
    ReorderLayoutItem(key = key, lazyIndex = lazyIndex, top = top, bottom = bottom)

  private fun layoutSnapshot(vararg items: ReorderLayoutItem<String>) =
    ReorderLayoutSnapshot(viewportTop = 0f, viewportBottom = 160f, items = items.toList())
}
