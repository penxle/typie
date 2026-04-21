package co.typie.screen.space.trash

import kotlin.test.Test
import kotlin.test.assertTrue

class TrashPlaceholderDataTest {
  @Test
  fun `root placeholder includes representative deleted entities`() {
    val data = trashRootPlaceholderData()

    assertTrue(data.site.deletedEntities.isNotEmpty())
  }

  @Test
  fun `folder placeholder includes representative deleted children`() {
    val data = trashFolderPlaceholderData()

    assertTrue(data.entity.deletedChildren.isNotEmpty())
  }
}
