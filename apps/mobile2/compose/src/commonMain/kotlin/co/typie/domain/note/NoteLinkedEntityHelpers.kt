package co.typie.domain.note

import androidx.compose.runtime.Composable
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.icons.Lucide
import co.typie.ui.EntityIconAppearance
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppTheme

internal fun NoteCard_note.linkedEntities(): List<NoteLinkedEntity_entity> = entities.map {
  it.noteLinkedEntity_entity
}

internal fun NoteLinkedEntity_entity.isFolder(): Boolean = node.onFolder != null

internal fun NoteLinkedEntity_entity.displayTitle(): String =
  when {
    node.onDocument != null -> node.onDocument.title.ifBlank { "(제목 없음)" }
    node.onFolder != null -> node.onFolder.name.ifBlank { "(이름 없음)" }
    else -> "(제목 없음)"
  }

internal fun NoteEntityPicker_entity.isFolder(): Boolean = node.onFolder != null

internal fun NoteEntityPicker_entity.displayTitle(): String =
  when {
    node.onDocument != null -> node.onDocument.title.ifBlank { "(제목 없음)" }
    node.onFolder != null -> node.onFolder.name.ifBlank { "(이름 없음)" }
    else -> "(제목 없음)"
  }

internal fun NoteEntityPicker_entity.displayPreviewText(): String? =
  when {
    node.onDocument != null ->
      node.onDocument.excerpt.takeIf { it.isNotBlank() }
        ?: node.onDocument.subtitle?.takeIf { it.isNotBlank() }
        ?: "문서"
    node.onFolder != null -> "폴더"
    else -> "문서"
  }

internal data class NoteParentFolderDisplay(
  val name: String,
  val icon: String?,
  val iconColor: String?,
)

internal fun NoteEntityPicker_entity.parentFolder(): NoteParentFolderDisplay? {
  val parentFolder = node.onDocument?.entity?.parent?.node?.onFolder ?: return null
  return NoteParentFolderDisplay(
    name = parentFolder.name.ifBlank { "(이름 없음)" },
    icon = parentFolder.entity.icon,
    iconColor = parentFolder.entity.iconColor,
  )
}

@Composable
internal fun NoteLinkedEntity_entity.iconAppearance(): EntityIconAppearance {
  return resolveEntityIconAppearance(
    iconName = icon,
    iconColor = iconColor,
    fallbackIcon = if (isFolder()) Lucide.Folder else Lucide.File,
    fallbackTint = AppTheme.colors.textMuted,
    colors = AppTheme.colors,
  )
}

@Composable
internal fun NoteEntityPicker_entity.iconAppearance(): EntityIconAppearance {
  return resolveEntityIconAppearance(
    iconName = icon,
    iconColor = iconColor,
    fallbackIcon = if (isFolder()) Lucide.Folder else Lucide.File,
    fallbackTint = AppTheme.colors.textSecondary,
    colors = AppTheme.colors,
  )
}
