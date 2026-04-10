package co.typie.screen.space.folder

import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

internal sealed interface FolderAction {
  data object Rename : FolderAction

  data object ChangeIcon : FolderAction

  data object OpenExternal : FolderAction

  data object Share : FolderAction

  data object Move : FolderAction

  data object Copy : FolderAction

  data object Cut : FolderAction

  data object Delete : FolderAction

  data object SelectMultiple : FolderAction

  data object StartReorder : FolderAction
}

internal data class FolderActionMenuItem(
  val icon: IconData,
  val label: String,
  val action: FolderAction,
  val trailingIcon: IconData? = null,
  val isDanger: Boolean = false,
)

internal data class FolderActionSection(val items: List<FolderActionMenuItem>)

internal data class FolderVisibilityPresentation(val label: String, val isShared: Boolean)

internal fun folderPrimaryActionSections(): List<FolderActionSection> {
  return listOf(
    FolderActionSection(
      items =
        listOf(
          FolderActionMenuItem(
            icon = Lucide.PenLine,
            label = "이름 변경",
            action = FolderAction.Rename,
          ),
          FolderActionMenuItem(
            icon = Lucide.Palette,
            label = "아이콘 변경",
            action = FolderAction.ChangeIcon,
          ),
        )
    ),
    FolderActionSection(
      items =
        listOf(
          FolderActionMenuItem(
            icon = Lucide.Globe,
            label = "스페이스에서 열기",
            trailingIcon = Lucide.ExternalLink,
            action = FolderAction.OpenExternal,
          ),
          FolderActionMenuItem(icon = Lucide.Blend, label = "공유 및 게시", action = FolderAction.Share),
        )
    ),
    FolderActionSection(
      items =
        listOf(
          FolderActionMenuItem(
            icon = Lucide.FolderSymlink,
            label = "다른 폴더로 옮기기",
            action = FolderAction.Move,
          ),
          FolderActionMenuItem(
            icon = Lucide.ClipboardCopy,
            label = "복사",
            action = FolderAction.Copy,
          ),
          FolderActionMenuItem(icon = Lucide.Scissors, label = "잘라내기", action = FolderAction.Cut),
        )
    ),
    FolderActionSection(
      items =
        listOf(
          FolderActionMenuItem(
            icon = Lucide.Trash2,
            label = "삭제",
            action = FolderAction.Delete,
            isDanger = true,
          )
        )
    ),
  )
}

internal fun folderVisibilityPresentation(
  visibility: EntityVisibility?,
  availability: EntityAvailability?,
): FolderVisibilityPresentation {
  return when {
    visibility == EntityVisibility.PUBLIC ->
      FolderVisibilityPresentation(label = "공개", isShared = true)
    visibility == EntityVisibility.UNLISTED && availability == EntityAvailability.UNLISTED ->
      FolderVisibilityPresentation(label = "링크 조회/편집 가능", isShared = true)
    visibility == EntityVisibility.UNLISTED ->
      FolderVisibilityPresentation(label = "링크 조회 가능", isShared = true)
    availability == EntityAvailability.UNLISTED ->
      FolderVisibilityPresentation(label = "링크 편집 가능", isShared = true)
    else -> FolderVisibilityPresentation(label = "비공개", isShared = false)
  }
}
