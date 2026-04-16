package co.typie.domain.entity

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class EntityMoveSheetModelsTest {
  @Test
  fun `view model key changes with destination`() {
    val rootKey = moveSheetViewModelKey(sourceId = "source-1", destinationEntityId = null)
    val folderKey = moveSheetViewModelKey(sourceId = "source-1", destinationEntityId = "folder-1")

    assertNotEquals(rootKey, folderKey)
    assertEquals(rootKey, moveSheetViewModelKey(sourceId = "source-1", destinationEntityId = null))
    assertEquals(
      folderKey,
      moveSheetViewModelKey(sourceId = "source-1", destinationEntityId = "folder-1"),
    )
  }
}
