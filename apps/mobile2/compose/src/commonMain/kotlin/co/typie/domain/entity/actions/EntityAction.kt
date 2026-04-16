package co.typie.domain.entity

import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

internal sealed interface EntityAction {
  data object Rename : EntityAction

  data object ChangeIcon : EntityAction

  data object OpenExternal : EntityAction

  data object Share : EntityAction

  data object Move : EntityAction

  data object Copy : EntityAction

  data object Cut : EntityAction

  data object Delete : EntityAction

  data object SelectMultiple : EntityAction

  data object StartReorder : EntityAction
}

internal data class EntityActionMenuItem(
  val icon: IconData,
  val label: String,
  val action: EntityAction,
  val trailingIcon: IconData? = null,
  val isDanger: Boolean = false,
)

internal data class EntityActionSection(val items: List<EntityActionMenuItem>)

internal fun entityItemActionSections(): List<EntityActionSection> {
  return listOf(
    EntityActionSection(
      items =
        listOf(
          EntityActionMenuItem(
            icon = Lucide.PenLine,
            label = "이름 변경",
            action = EntityAction.Rename,
          ),
          EntityActionMenuItem(
            icon = Lucide.Palette,
            label = "아이콘 변경",
            action = EntityAction.ChangeIcon,
          ),
        )
    ),
    EntityActionSection(
      items =
        listOf(
          EntityActionMenuItem(
            icon = Lucide.Globe,
            label = "스페이스에서 열기",
            trailingIcon = Lucide.ExternalLink,
            action = EntityAction.OpenExternal,
          ),
          EntityActionMenuItem(icon = Lucide.Blend, label = "공유 및 게시", action = EntityAction.Share),
        )
    ),
    EntityActionSection(
      items =
        listOf(
          EntityActionMenuItem(
            icon = Lucide.FolderSymlink,
            label = "다른 폴더로 옮기기",
            action = EntityAction.Move,
          ),
          EntityActionMenuItem(
            icon = Lucide.ClipboardCopy,
            label = "복사",
            action = EntityAction.Copy,
          ),
          EntityActionMenuItem(icon = Lucide.Scissors, label = "잘라내기", action = EntityAction.Cut),
        )
    ),
    EntityActionSection(
      items =
        listOf(
          EntityActionMenuItem(
            icon = Lucide.Trash2,
            label = "삭제",
            action = EntityAction.Delete,
            isDanger = true,
          )
        )
    ),
  )
}
