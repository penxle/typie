package co.typie.ui.component

import co.typie.entity_transfer.EntityTransferSource
import co.typie.screen.space.folder.toTransferSource
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EntityListRowsTest {
  @Test
  fun `formatFolderMetadataSummary includes grouped character count`() {
    assertEquals(
      "폴더 1,234개 · 문서 56개 · 총 78,901자",
      formatFolderMetadataSummary(
        folderCount = 1234,
        documentCount = 56,
        characterCount = 78901,
      ),
    )
  }

  @Test
  fun `formatFolderMetadataSummary keeps total characters when folder is empty`() {
    assertEquals(
      "총 0자",
      formatFolderMetadataSummary(
        folderCount = 0,
        documentCount = 0,
        characterCount = 0,
      ),
    )
  }

  @Test
  fun `folder item keeps entity id for transfer and folder id for folder mutations`() {
    val item = EntityListItem.Folder(
      id = "entity-1",
      folderId = "folder-1",
      iconName = "folder",
      iconColor = "gray",
      name = "프로젝트",
      folderCount = 0,
      documentCount = 0,
      depth = 3,
      maxDescendantFoldersDepth = 4,
    )

    assertEquals("folder-1", item.folderId)
    assertEquals(
      EntityTransferSource.Folder(
        id = "entity-1",
        title = "프로젝트",
        depth = 3,
        maxDescendantFoldersDepth = 4,
      ),
      item.toTransferSource(),
    )
  }

  @Test
  fun `row behavior keeps full opacity when only interaction is disabled`() {
    val behavior = entityListRowBehavior(
      enabled = true,
      interactive = false,
      opacity = 0.5f,
    )

    assertEquals(0.5f, behavior.alpha)
    assertFalse(behavior.isInteractive)
  }

  @Test
  fun `row behavior dims disabled rows and blocks interaction`() {
    val behavior = entityListRowBehavior(
      enabled = false,
      interactive = true,
      opacity = 1f,
    )

    assertEquals(0.48f, behavior.alpha)
    assertFalse(behavior.isInteractive)
  }

  @Test
  fun `row behavior keeps enabled rows interactive`() {
    val behavior = entityListRowBehavior(
      enabled = true,
      interactive = true,
      opacity = 1f,
    )

    assertEquals(1f, behavior.alpha)
    assertTrue(behavior.isInteractive)
  }
}
