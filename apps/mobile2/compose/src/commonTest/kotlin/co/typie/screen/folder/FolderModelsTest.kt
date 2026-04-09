package co.typie.screen.folder

import co.typie.ui.component.EntityListItem
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class FolderModelsTest {
  @Test
  fun `calculateFolderReorderOrdersFromOrderedKeys returns upper order when moved to first position`() {
    val result = calculateFolderReorderOrdersFromOrderedKeys(
      items = listOf(
        folderChild(id = "a", order = "10"),
        folderChild(id = "b", order = "20"),
        folderChild(id = "c", order = "30"),
      ),
      orderedKeys = listOf("c", "a", "b"),
      movedKey = "c",
    )

    assertEquals(
      FolderReorderOrders(
        lowerOrder = null,
        upperOrder = "10",
      ),
      result,
    )
  }

  @Test
  fun `calculateFolderReorderOrdersFromOrderedKeys returns adjacent orders when moved to middle position`() {
    val result = calculateFolderReorderOrdersFromOrderedKeys(
      items = listOf(
        folderChild(id = "a", order = "10"),
        folderChild(id = "b", order = "20"),
        folderChild(id = "c", order = "30"),
      ),
      orderedKeys = listOf("a", "c", "b"),
      movedKey = "c",
    )

    assertEquals(
      FolderReorderOrders(
        lowerOrder = "10",
        upperOrder = "20",
      ),
      result,
    )
  }

  @Test
  fun `calculateFolderReorderOrdersFromOrderedKeys returns lower order when moved to last position`() {
    val result = calculateFolderReorderOrdersFromOrderedKeys(
      items = listOf(
        folderChild(id = "a", order = "10"),
        folderChild(id = "b", order = "20"),
        folderChild(id = "c", order = "30"),
      ),
      orderedKeys = listOf("b", "c", "a"),
      movedKey = "a",
    )

    assertEquals(
      FolderReorderOrders(
        lowerOrder = "30",
        upperOrder = null,
      ),
      result,
    )
  }

  @Test
  fun `calculateFolderReorderOrdersFromOrderedKeys returns null when moved key is missing`() {
    val result = calculateFolderReorderOrdersFromOrderedKeys(
      items = listOf(
        folderChild(id = "a", order = "10"),
        folderChild(id = "b", order = "20"),
      ),
      orderedKeys = listOf("a", "b"),
      movedKey = "c",
    )

    assertNull(result)
  }

  @Test
  fun `calculateFolderReorderOrdersFromOrderedKeys returns null when ordered keys are incomplete`() {
    val result = calculateFolderReorderOrdersFromOrderedKeys(
      items = listOf(
        folderChild(id = "a", order = "10"),
        folderChild(id = "b", order = "20"),
        folderChild(id = "c", order = "30"),
      ),
      orderedKeys = listOf("a", "c"),
      movedKey = "c",
    )

    assertNull(result)
  }

  private fun folderChild(
    id: String,
    order: String,
  ): NormalizedFolderChild {
    return NormalizedFolderChild(
      id = id,
      order = order,
      item = EntityListItem.Folder(
        id = id,
        iconName = "folder",
        iconColor = "gray",
        name = id,
        folderCount = 0,
        documentCount = 0,
      ),
    )
  }
}
