package co.typie.entity_transfer

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

const val EntityTransferMaxDepth = 100

sealed interface EntityTransferSource {
  val id: String
  val title: String
  val depth: Int

  val transferActionLabel: String
    get() = "다른 폴더로 옮기기"

  val transferActionIcon: IconData

  val internalDepthContribution: Int

  fun canTransferIntoDestinationDepth(destinationDepth: Int): Boolean {
    val targetDepth = destinationDepth + 1
    return targetDepth + internalDepthContribution <= EntityTransferMaxDepth
  }

  data class Folder(
    override val id: String,
    override val title: String,
    override val depth: Int,
    val maxDescendantFoldersDepth: Int,
  ) : EntityTransferSource {
    override val transferActionIcon: IconData = Lucide.FolderSymlink
    override val internalDepthContribution: Int
      get() = maxDescendantFoldersDepth - depth + 1
  }

  data class Document(
    override val id: String,
    override val title: String,
    override val depth: Int,
  ) : EntityTransferSource {
    override val transferActionIcon: IconData = Lucide.FileSymlink
    override val internalDepthContribution: Int = 0
  }
}
