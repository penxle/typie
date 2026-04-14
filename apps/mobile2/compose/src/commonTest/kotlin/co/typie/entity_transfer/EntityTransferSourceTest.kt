package co.typie.entity_transfer

import co.typie.domain.entity_transfer.EntityTransferSource
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EntityTransferSourceTest {

  @Test
  fun folderInternalDepthContributionUsesMaxDescendantDepth() {
    val source =
      EntityTransferSource.Folder(
        id = "folder-1",
        title = "프로젝트",
        depth = 4,
        maxDescendantFoldersDepth = 6,
      )

    assertTrue(source.internalDepthContribution == 3)
  }

  @Test
  fun documentInternalDepthContributionIsZero() {
    val source = EntityTransferSource.Document(id = "document-1", title = "문서", depth = 7)

    assertTrue(source.internalDepthContribution == 0)
  }

  @Test
  fun folderTransferDepthValidationMatchesFlutterParity() {
    val source =
      EntityTransferSource.Folder(
        id = "folder-1",
        title = "프로젝트",
        depth = 4,
        maxDescendantFoldersDepth = 6,
      )

    assertTrue(source.canTransferIntoDestinationDepth(destinationDepth = 96))
    assertFalse(source.canTransferIntoDestinationDepth(destinationDepth = 97))
  }
}
