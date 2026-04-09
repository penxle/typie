package co.typie.screen.entity_move

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class MoveSourceEntityTest {

  @Test
  fun folderInternalDepthContributionUsesMaxDescendantDepth() {
    val source = MoveSourceEntity.Folder(
      id = "folder-1",
      title = "프로젝트",
      depth = 4,
      maxDescendantFoldersDepth = 6,
    )

    assertTrue(source.internalDepthContribution == 3)
  }

  @Test
  fun documentInternalDepthContributionIsZero() {
    val source = MoveSourceEntity.Document(
      id = "document-1",
      title = "문서",
      depth = 7,
    )

    assertTrue(source.internalDepthContribution == 0)
  }

  @Test
  fun folderMoveDepthValidationMatchesFlutterParity() {
    val source = MoveSourceEntity.Folder(
      id = "folder-1",
      title = "프로젝트",
      depth = 4,
      maxDescendantFoldersDepth = 6,
    )

    assertTrue(source.canMoveIntoDestinationDepth(destinationDepth = 96))
    assertFalse(source.canMoveIntoDestinationDepth(destinationDepth = 97))
  }
}
