package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.combinedClickable
import co.typie.ext.comma
import co.typie.ext.pressScale
import co.typie.ext.separated
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Instant

sealed interface EntityListItem {
  val id: String
  val iconName: String
  val iconColor: String

  data class Document(
    override val id: String,
    val documentId: String,
    override val iconName: String,
    override val iconColor: String,
    val slug: String,
    val title: String,
    val subtitle: String?,
    val excerpt: String,
    val updatedAt: Instant,
    val siteName: String? = null,
    val ancestorFolderNames: List<String> = emptyList(),
    val depth: Int = 0,
    val url: String = "",
    val visibility: EntityVisibility? = null,
    val availability: EntityAvailability? = null,
    val characterCount: Int = 0,
  ) : EntityListItem

  data class Folder(
    override val id: String,
    val folderId: String,
    override val iconName: String,
    override val iconColor: String,
    val name: String,
    val folderCount: Int,
    val documentCount: Int,
    val siteName: String? = null,
    val ancestorFolderNames: List<String> = emptyList(),
    val depth: Int = 0,
    val url: String = "",
    val visibility: EntityVisibility? = null,
    val availability: EntityAvailability? = null,
    val characterCount: Int = 0,
    val maxDescendantFoldersDepth: Int = 0,
    val thumbnailUrl: String? = null,
  ) : EntityListItem
}

fun formatSpaceSummary(folderCount: Int, documentCount: Int): String =
  formatEntitySummary(folderCount, documentCount, emptyText = "비어 있는 스페이스")

fun formatFolderSummary(folderCount: Int, documentCount: Int): String =
  formatEntitySummary(folderCount, documentCount, emptyText = "빈 폴더")

fun formatFolderMetadataSummary(folderCount: Int, documentCount: Int, characterCount: Int): String {
  val parts = buildList {
    if (folderCount > 0) add("폴더 ${folderCount.comma}개")
    if (documentCount > 0) add("문서 ${documentCount.comma}개")
    add("총 ${characterCount.comma}자")
  }

  return parts.joinToString(" · ")
}

fun EntityListItem.Folder.breadcrumbNames(): List<String> {
  return buildList {
    siteName?.takeIf { it.isNotBlank() }?.let(::add)
    addAll(ancestorFolderNames)
  }
}

fun EntityListItem.Document.breadcrumbNames(): List<String> {
  return buildList {
    siteName?.takeIf { it.isNotBlank() }?.let(::add)
    addAll(ancestorFolderNames)
  }
}

fun formatFolderRowSummary(folderCount: Int, documentCount: Int): String {
  if (folderCount == 0 && documentCount == 0) {
    return "빈 폴더"
  }

  if (folderCount == 0) {
    return "문서 ${documentCount.comma}개"
  }

  return "폴더 ${folderCount.comma}개 · 문서 ${documentCount.comma}개"
}

@Composable
fun EntityListCard(
  items: List<EntityListItem>,
  emptyMessage: String,
  selectionState: co.typie.ui.component.entitycontainer.EntityContainerSelectionState =
    co.typie.ui.component.entitycontainer.EntityContainerSelectionState(),
  dimmedItemIds: Set<String> = emptySet(),
  modifier: Modifier = Modifier,
  onDocumentClick: suspend (slug: String) -> Unit,
  onDocumentLongPress: (suspend (item: EntityListItem.Document) -> Unit)? = null,
  onFolderClick: suspend (entityId: String) -> Unit,
  onFolderLongPress: (suspend (item: EntityListItem.Folder) -> Unit)? = null,
  onSelectionToggle: suspend (itemId: String) -> Unit = {},
) {
  if (items.isEmpty()) {
    Box(
      modifier =
        modifier
          .fillMaxWidth()
          .height(110.dp)
          .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md)),
      contentAlignment = Alignment.Center,
    ) {
      Text(emptyMessage, style = AppTheme.typography.action, color = AppTheme.colors.textTertiary)
    }
    return
  }

  CardSurface(modifier = modifier.fillMaxWidth()) {
    Column(Modifier.fillMaxWidth()) {
      items.separated(separator = { CardDivider() }) { item ->
        when (item) {
          is EntityListItem.Document ->
            EntityListDocumentRow(
              item = item,
              selected = item.id in selectionState.selectedIds,
              showSelectionControls = selectionState.isSelecting,
              opacity = if (item.id in dimmedItemIds) 0.5f else 1f,
              onLongPress = onDocumentLongPress?.let { handler -> { handler(item) } },
              onClick = {
                if (selectionState.isSelecting) {
                  onSelectionToggle(item.id)
                } else {
                  onDocumentClick(item.slug)
                }
              },
            )

          is EntityListItem.Folder ->
            EntityListFolderRow(
              item = item,
              selected = item.id in selectionState.selectedIds,
              showSelectionControls = selectionState.isSelecting,
              opacity = if (item.id in dimmedItemIds) 0.5f else 1f,
              onLongPress = onFolderLongPress?.let { handler -> { handler(item) } },
              onClick = {
                if (selectionState.isSelecting) {
                  onSelectionToggle(item.id)
                } else {
                  onFolderClick(item.id)
                }
              },
            )
        }
      }
    }
  }
}

internal data class EntityListRowBehavior(val alpha: Float, val isInteractive: Boolean)

internal fun entityListRowBehavior(
  enabled: Boolean,
  interactive: Boolean,
  opacity: Float,
): EntityListRowBehavior {
  return EntityListRowBehavior(
    alpha = (if (enabled) 1f else 0.48f) * opacity,
    isInteractive = enabled && interactive,
  )
}

@Composable
fun EntityListDocumentRow(
  item: EntityListItem.Document,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  DocumentRowContent(
    modifier = modifier,
    iconName = item.iconName,
    iconColor = item.iconColor,
    title = item.title,
    subtitle = item.subtitle,
    excerpt = item.excerpt,
    updatedAt = item.updatedAt,
    enabled = enabled,
    interactive = interactive,
    opacity = opacity,
    selected = selected,
    showSelectionControls = showSelectionControls,
    onLongPress = onLongPress,
    onClick = onClick,
  )
}

@Composable
fun EntityListFolderRow(
  item: EntityListItem.Folder,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  FolderRowContent(
    title = AnnotatedString(item.name),
    iconName = item.iconName,
    iconColor = item.iconColor,
    metaText =
      formatFolderRowSummary(folderCount = item.folderCount, documentCount = item.documentCount),
    enabled = enabled,
    interactive = interactive,
    opacity = opacity,
    selected = selected,
    showSelectionControls = showSelectionControls,
    modifier = modifier,
    onLongPress = onLongPress,
    onClick = onClick,
  )
}

@Composable
fun SearchFolderRow(
  title: AnnotatedString,
  iconName: String,
  iconColor: String,
  folderCount: Int,
  documentCount: Int,
  modifier: Modifier = Modifier,
  onClick: suspend () -> Unit,
) {
  FolderRowContent(
    title = title,
    iconName = iconName,
    iconColor = iconColor,
    metaText = formatFolderRowSummary(folderCount = folderCount, documentCount = documentCount),
    modifier = modifier,
    onClick = onClick,
  )
}

@Composable
fun TrashDocumentRow(
  title: String,
  subtitle: String?,
  excerpt: String?,
  updatedAt: Instant?,
  iconName: String,
  iconColor: String,
  modifier: Modifier = Modifier,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  DocumentRowContent(
    modifier = modifier,
    iconName = iconName,
    iconColor = iconColor,
    title = title,
    subtitle = subtitle,
    excerpt = excerpt.orEmpty(),
    updatedAt = updatedAt,
    onLongPress = onLongPress,
    onClick = onClick,
  )
}

@Composable
fun TrashFolderRow(
  title: String,
  iconName: String,
  iconColor: String,
  modifier: Modifier = Modifier,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  FolderRowContent(
    title = AnnotatedString(title),
    iconName = iconName,
    iconColor = iconColor,
    metaText = "삭제된 폴더",
    modifier = modifier,
    onLongPress = onLongPress,
    onClick = onClick,
  )
}

@Composable
private fun DocumentRowContent(
  iconName: String,
  iconColor: String,
  title: String,
  subtitle: String?,
  excerpt: String,
  updatedAt: Instant?,
  modifier: Modifier = Modifier,
  emptyExcerptText: String? = "(내용 없음)",
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  val metaColor = AppTheme.colors.textMuted
  val entityIcon =
    resolveEntityIconAppearance(
      iconName = iconName,
      iconColor = iconColor,
      fallbackIcon = Lucide.File,
      fallbackTint = metaColor,
      colors = AppTheme.colors,
    )
  val resolvedSubtitle = subtitle?.takeIf { it.isNotBlank() }
  val titleText = buildAnnotatedString {
    append(title)

    if (resolvedSubtitle != null) {
      pushStyle(SpanStyle(color = metaColor))
      append(" — ")
      append(resolvedSubtitle)
      pop()
    }
  }
  val resolvedExcerpt = excerpt.takeIf { it.isNotEmpty() } ?: emptyExcerptText

  EntityListRowFrame(
    modifier = modifier,
    icon = entityIcon.icon,
    iconTint = entityIcon.tint,
    enabled = enabled,
    interactive = interactive,
    opacity = opacity,
    selected = selected,
    showSelectionControls = showSelectionControls,
    onLongPress = onLongPress,
    onClick = onClick,
  ) {
    Row(verticalAlignment = Alignment.CenterVertically) {
      Text(
        text = titleText,
        style = AppTheme.typography.label,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier.weight(1f),
      )

      if (updatedAt != null) {
        Spacer(Modifier.width(8.dp))

        Text(updatedAt.timeAgo(), style = AppTheme.typography.caption, color = metaColor)
      }
    }

    if (resolvedExcerpt != null) {
      Spacer(Modifier.height(4.dp))

      Text(
        resolvedExcerpt,
        style = AppTheme.typography.caption,
        color = metaColor,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun FolderRowContent(
  title: AnnotatedString,
  iconName: String,
  iconColor: String,
  metaText: String,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  val metaColor = AppTheme.colors.textMuted
  val entityIcon =
    resolveEntityIconAppearance(
      iconName = iconName,
      iconColor = iconColor,
      fallbackIcon = Lucide.Folder,
      fallbackTint = AppTheme.colors.brand,
      colors = AppTheme.colors,
    )

  EntityListRowFrame(
    modifier = modifier,
    icon = entityIcon.icon,
    iconTint = entityIcon.tint,
    enabled = enabled,
    interactive = interactive,
    opacity = opacity,
    selected = selected,
    showSelectionControls = showSelectionControls,
    onLongPress = onLongPress,
    onClick = onClick,
    trailing =
      if (showSelectionControls) null
      else {
        {
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(18.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }
      },
  ) {
    Text(
      text = title,
      style = AppTheme.typography.label,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Spacer(Modifier.height(4.dp))

    Text(
      text = metaText,
      style = AppTheme.typography.caption,
      color = metaColor,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun EntityListRowFrame(
  icon: co.typie.ui.icon.IconData,
  iconTint: Color,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  onLongPress: (suspend () -> Unit)? = null,
  trailing: (@Composable () -> Unit)? = null,
  onClick: suspend () -> Unit,
  content: @Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
  val behavior =
    entityListRowBehavior(enabled = enabled, interactive = interactive, opacity = opacity)

  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .background(
            if (showSelectionControls && selected) AppTheme.colors.brandSubtle
            else Color.Transparent
          )
          .alpha(behavior.alpha)
          .then(
            if (!behavior.isInteractive) {
              Modifier
            } else if (onLongPress != null) {
              Modifier.combinedClickable(onClick = onClick, onLongClick = onLongPress)
            } else {
              Modifier.clickable(onClick)
            }
          )
          .then(if (behavior.isInteractive) Modifier.pressScale() else Modifier)
          .padding(horizontal = 16.dp, vertical = 12.dp),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      if (showSelectionControls) {
        Icon(
          icon = if (selected) Lucide.SquareCheck else Lucide.Square,
          modifier = Modifier.size(18.dp),
          tint = if (selected) AppTheme.colors.brand else AppTheme.colors.textTertiary,
        )
      }

      Icon(icon = icon, modifier = Modifier.size(18.dp), tint = iconTint)

      Column(modifier = Modifier.weight(1f), content = content)

      if (trailing != null) {
        trailing()
      }
    }
  }
}

private fun formatEntitySummary(folderCount: Int, documentCount: Int, emptyText: String): String {
  val parts = buildList {
    if (folderCount > 0) add("폴더 ${folderCount.comma}개")
    if (documentCount > 0) add("문서 ${documentCount.comma}개")
  }

  return if (parts.isEmpty()) emptyText else parts.joinToString(" · ")
}
