package co.typie.ui.component.entity_container

import co.typie.ui.component.EntityListItem
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EntityContainerModelsTest {
  @Test
  fun `calculateEntityReorderOrdersFromOrderedKeys returns upper order when moved to first position`() {
    val result = calculateEntityReorderOrdersFromOrderedKeys(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("c", "a", "b"),
      movedKey = "c",
    )

    assertEquals(
      EntityReorderOrders(
        lowerOrder = null,
        upperOrder = "10",
      ),
      result,
    )
  }

  @Test
  fun `calculateEntityReorderOrdersFromOrderedKeys returns adjacent orders when moved to middle position`() {
    val result = calculateEntityReorderOrdersFromOrderedKeys(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("a", "c", "b"),
      movedKey = "c",
    )

    assertEquals(
      EntityReorderOrders(
        lowerOrder = "10",
        upperOrder = "20",
      ),
      result,
    )
  }

  @Test
  fun `calculateEntityReorderOrdersFromOrderedKeys returns lower order when moved to last position`() {
    val result = calculateEntityReorderOrdersFromOrderedKeys(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("b", "c", "a"),
      movedKey = "a",
    )

    assertEquals(
      EntityReorderOrders(
        lowerOrder = "30",
        upperOrder = null,
      ),
      result,
    )
  }

  @Test
  fun `calculateEntityReorderOrdersFromOrderedKeys returns null when moved key is missing`() {
    val result = calculateEntityReorderOrdersFromOrderedKeys(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
      ),
      orderedKeys = listOf("a", "b"),
      movedKey = "c",
    )

    assertNull(result)
  }

  @Test
  fun `calculateEntityReorderOrdersFromOrderedKeys returns null when ordered keys are incomplete`() {
    val result = calculateEntityReorderOrdersFromOrderedKeys(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("a", "c"),
      movedKey = "c",
    )

    assertNull(result)
  }

  @Test
  fun `displayOrderedEntityItems returns ordered items when keys cover the full set`() {
    val result = displayOrderedEntityItems(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("c", "a", "b"),
    )

    assertEquals(listOf("c", "a", "b"), result.map { it.id })
  }

  @Test
  fun `displayOrderedEntityItems falls back to server order when ordered keys are incomplete`() {
    val result = displayOrderedEntityItems(
      items = listOf(
        orderedItem(id = "a", order = "10"),
        orderedItem(id = "b", order = "20"),
        orderedItem(id = "c", order = "30"),
      ),
      orderedKeys = listOf("c", "a"),
    )

    assertEquals(listOf("a", "b", "c"), result.map { it.id })
  }

  private fun orderedItem(
    id: String,
    order: String,
  ): OrderedEntityItem {
    return OrderedEntityItem(
      id = id,
      order = order,
      item = EntityListItem.Folder(
        id = id,
        folderId = id,
        iconName = "folder",
        iconColor = "gray",
        name = id,
        folderCount = 0,
        documentCount = 0,
      ),
    )
  }
}
