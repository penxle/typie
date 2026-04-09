package co.typie.screen.entity_move

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

internal const val MoveEntityMaxDepth = 100

sealed interface MoveSourceEntity {
  val id: String
  val title: String
  val depth: Int

  val displayTitle: String
    get() = title

  val moveActionLabel: String
    get() = "다른 폴더로 옮기기"

  val moveActionIcon: IconData

  val internalDepthContribution: Int

  fun canMoveIntoDestinationDepth(destinationDepth: Int): Boolean {
    val targetDepth = destinationDepth + 1
    return targetDepth + internalDepthContribution <= MoveEntityMaxDepth
  }

  data class Folder(
    override val id: String,
    override val title: String,
    override val depth: Int,
    val maxDescendantFoldersDepth: Int,
  ) : MoveSourceEntity {
    override val moveActionIcon: IconData = Lucide.FolderSymlink
    override val internalDepthContribution: Int
      get() = maxDescendantFoldersDepth - depth + 1
  }

  data class Document(
    override val id: String,
    override val title: String,
    override val depth: Int,
  ) : MoveSourceEntity {
    override val moveActionIcon: IconData = Lucide.FileSymlink
    override val internalDepthContribution: Int = 0
  }
}
