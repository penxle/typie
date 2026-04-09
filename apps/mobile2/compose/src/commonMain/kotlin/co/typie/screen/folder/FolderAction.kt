package co.typie.screen.folder

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

internal sealed interface FolderAction {
  data object Rename : FolderAction
  data object ChangeIcon : FolderAction
  data object OpenExternal : FolderAction
  data object Share : FolderAction
  data object Copy : FolderAction
  data object Cut : FolderAction
  data object Delete : FolderAction
  data object SelectMultiple : FolderAction
  data object StartReorder : FolderAction
}

internal data class FolderTopBarActionItem(
  val icon: IconData,
  val label: String,
  val action: FolderAction,
  val trailingIcon: IconData? = null,
  val isDanger: Boolean = false,
)

internal data class FolderVisibilityPresentation(
  val label: String,
  val isShared: Boolean,
)

internal fun folderTopBarCenterActions(): List<FolderTopBarActionItem> {
  return listOf(
    FolderTopBarActionItem(
      icon = Lucide.PenLine,
      label = "이름 변경",
      action = FolderAction.Rename,
    ),
    FolderTopBarActionItem(
      icon = Lucide.Palette,
      label = "아이콘 변경",
      action = FolderAction.ChangeIcon,
    ),
    FolderTopBarActionItem(
      icon = Lucide.Globe,
      label = "스페이스에서 열기",
      trailingIcon = Lucide.ExternalLink,
      action = FolderAction.OpenExternal,
    ),
    FolderTopBarActionItem(
      icon = Lucide.Blend,
      label = "공유 및 게시",
      action = FolderAction.Share,
    ),
    FolderTopBarActionItem(
      icon = Lucide.ClipboardCopy,
      label = "복사",
      action = FolderAction.Copy,
    ),
    FolderTopBarActionItem(
      icon = Lucide.Scissors,
      label = "잘라내기",
      action = FolderAction.Cut,
    ),
    FolderTopBarActionItem(
      icon = Lucide.Trash2,
      label = "삭제",
      action = FolderAction.Delete,
      isDanger = true,
    ),
  )
}

internal fun folderVisibilityPresentation(
  visibilityName: String?,
  availabilityName: String?,
): FolderVisibilityPresentation {
  return when {
    visibilityName == "PUBLIC" -> FolderVisibilityPresentation(label = "공개", isShared = true)
    visibilityName == "UNLISTED" && availabilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 조회/편집 가능", isShared = true)
    visibilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 조회 가능", isShared = true)
    availabilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 편집 가능", isShared = true)
    else -> FolderVisibilityPresentation(label = "비공개", isShared = false)
  }
}
