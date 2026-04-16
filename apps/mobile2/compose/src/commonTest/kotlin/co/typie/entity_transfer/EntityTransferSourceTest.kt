package co.typie.entity_transfer

import co.typie.domain.entitytransfer.EntityTransferSource
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EntityTransferSourceTest {

  @Test
  fun folderSubtreeFolderDepthUsesMaxDescendantDepth() {
    val source =
      EntityTransferSource.Folder(
        id = "folder-1",
        title = "프로젝트",
        depth = 4,
        maxDescendantFoldersDepth = 6,
      )

    assertTrue(source.subtreeFolderDepth == 3)
  }

  @Test
  fun documentSubtreeFolderDepthIsZero() {
    val source = EntityTransferSource.Document(id = "document-1", title = "문서", depth = 7)

    assertTrue(source.subtreeFolderDepth == 0)
  }

  @Test
  fun folderCanMoveToDepthMatchesFlutterParity() {
    val source =
      EntityTransferSource.Folder(
        id = "folder-1",
        title = "프로젝트",
        depth = 4,
        maxDescendantFoldersDepth = 6,
      )

    assertTrue(source.canMoveToDepth(destinationDepth = 96))
    assertFalse(source.canMoveToDepth(destinationDepth = 97))
  }
}
