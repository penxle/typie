package co.typie.domain.entity

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.combinedClickable
import co.typie.ext.pressScale
import co.typie.graphql.fragment.EntityParentMeta_folder
import co.typie.graphql.fragment.EntityRowDocument_document
import co.typie.graphql.fragment.EntityRowFolder_folder
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

object EntityRowDefaults {
  val ContentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp)
  val ItemSpacing = 12.dp
  val MetaSpacing = 4.dp
  val MetaIconSize = 12.dp
  val IconSize = 18.dp
  val LineSpacing = 4.dp
}

internal sealed interface EntityRowScopeEntry

internal data class EntityRowParentMetaEntry(val folder: EntityParentMeta_folder) :
  EntityRowScopeEntry

internal data class EntityRowTitleEntry(
  val title: EntityRowText,
  val subtitle: EntityRowText?,
  val trailingText: String?,
) : EntityRowScopeEntry

internal data class EntityRowSupportingEntry(val text: EntityRowText, val maxLines: Int) :
  EntityRowScopeEntry

internal data class EntityRowCustomEntry(val content: @Composable ColumnScope.() -> Unit) :
  EntityRowScopeEntry

internal sealed interface EntityRowText {
  val value: String

  data class Plain(override val value: String) : EntityRowText

  data class Rich(val annotated: AnnotatedString) : EntityRowText {
    override val value: String
      get() = annotated.text
  }
}

class EntityRowScope {
  @PublishedApi internal val entries = mutableListOf<EntityRowScopeEntry>()

  fun parentMeta(folder: EntityParentMeta_folder?) {
    if (folder != null) {
      entries.add(EntityRowParentMetaEntry(folder))
    }
  }

  fun title(title: String, subtitle: String? = null, trailingText: String? = null) {
    entries.add(
      EntityRowTitleEntry(
        title = EntityRowText.Plain(title),
        subtitle = subtitle?.takeIf(String::isNotBlank)?.let(EntityRowText::Plain),
        trailingText = trailingText?.takeIf(String::isNotBlank),
      )
    )
  }

  fun title(
    title: AnnotatedString,
    subtitle: AnnotatedString? = null,
    trailingText: String? = null,
  ) {
    entries.add(
      EntityRowTitleEntry(
        title = EntityRowText.Rich(title),
        subtitle = subtitle?.takeIf { it.text.isNotBlank() }?.let(EntityRowText::Rich),
        trailingText = trailingText?.takeIf(String::isNotBlank),
      )
    )
  }

  fun supporting(text: String, maxLines: Int = 1) {
    if (text.isBlank()) {
      return
    }

    entries.add(EntityRowSupportingEntry(text = EntityRowText.Plain(text), maxLines = maxLines))
  }

  fun supporting(text: AnnotatedString, maxLines: Int = 1) {
    if (text.text.isBlank()) {
      return
    }

    entries.add(EntityRowSupportingEntry(text = EntityRowText.Rich(text), maxLines = maxLines))
  }

  fun custom(content: @Composable ColumnScope.() -> Unit) {
    entries.add(EntityRowCustomEntry(content = content))
  }

  fun documentTitle(document: EntityRowDocument_document, trailingText: String? = null) {
    title(
      title = formatDocumentTitle(document.title),
      subtitle = document.subtitle,
      trailingText = trailingText,
    )
  }

  fun documentExcerpt(
    document: EntityRowDocument_document,
    maxLines: Int = 1,
    emptyText: String = EMPTY_ENTITY_EXCERPT_TEXT,
  ) {
    supporting(text = formatEntityExcerpt(document.excerpt, emptyText), maxLines = maxLines)
  }

  fun folderTitle(folder: EntityRowFolder_folder, emptyText: String = UNNAMED_FOLDER_TEXT) {
    title(title = formatFolderName(folder.name, emptyText))
  }

  fun folderSummary(folder: EntityRowFolder_folder) {
    supporting(text = formatFolderRowSummary(folder.folderCount, folder.documentCount))
  }
}

@Composable
fun EntityRow(
  entity: EntityRow_entity,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  backgroundColor: Color = Color.Transparent,
  contentPadding: PaddingValues = EntityRowDefaults.ContentPadding,
  leading: (@Composable () -> Unit)? = null,
  trailing: (@Composable () -> Unit)? = null,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit = {},
  content: EntityRowScope.() -> Unit,
) {
  val alpha = (if (enabled) 1f else 0.48f) * opacity
  val isInteractive = enabled && interactive
  val iconAppearance = entity.entityIcon_entity.iconAppearance
  val entries = EntityRowScope().apply(content).entries

  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .background(backgroundColor)
          .graphicsLayer { this.alpha = alpha }
          .then(
            if (!isInteractive) {
              Modifier
            } else if (onLongPress != null) {
              Modifier.combinedClickable(onClick = onClick, onLongClick = onLongPress)
            } else {
              Modifier.clickable(onClick)
            }
          )
          .then(if (isInteractive) Modifier.pressScale() else Modifier)
          .padding(contentPadding),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(EntityRowDefaults.ItemSpacing),
    ) {
      if (leading != null) {
        leading()
      }

      Icon(
        icon = iconAppearance.icon,
        modifier = Modifier.size(EntityRowDefaults.IconSize),
        tint = iconAppearance.tint,
      )

      Column(modifier = Modifier.weight(1f)) {
        entries.forEachIndexed { index, entry ->
          if (index > 0) {
            Spacer(Modifier.size(EntityRowDefaults.LineSpacing))
          }

          when (entry) {
            is EntityRowParentMetaEntry -> EntityRowParentMeta(folder = entry.folder)
            is EntityRowTitleEntry -> EntityRowTitle(entry)
            is EntityRowSupportingEntry -> EntityRowSupporting(entry)
            is EntityRowCustomEntry -> entry.content(this)
          }
        }
      }

      if (trailing != null) {
        trailing()
      }
    }
  }
}

@Composable
private fun EntityRowTitle(entry: EntityRowTitleEntry) {
  Row(verticalAlignment = Alignment.CenterVertically) {
    Text(
      text = entry.buildText(),
      style = AppTheme.typography.label,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
      modifier = Modifier.weight(1f),
    )

    entry.trailingText?.let { trailingText ->
      Spacer(Modifier.size(8.dp))
      Text(
        text = trailingText,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textHint,
      )
    }
  }
}

@Composable
private fun EntityRowSupporting(entry: EntityRowSupportingEntry) {
  when (val text = entry.text) {
    is EntityRowText.Plain ->
      Text(
        text = text.value,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textHint,
        maxLines = entry.maxLines,
        overflow = TextOverflow.Ellipsis,
      )

    is EntityRowText.Rich ->
      Text(
        text = text.annotated,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textHint,
        maxLines = entry.maxLines,
        overflow = TextOverflow.Ellipsis,
      )
  }
}

@Composable
private fun EntityRowTitleEntry.buildText(): AnnotatedString {
  return buildAnnotatedString {
    append(title.toAnnotatedString())

    subtitle?.let { subtitle ->
      pushStyle(SpanStyle(color = AppTheme.colors.textHint))
      append(" — ")
      pop()
      append(subtitle.toAnnotatedString())
    }
  }
}

private fun EntityRowText.toAnnotatedString(): AnnotatedString {
  return when (this) {
    is EntityRowText.Plain -> AnnotatedString(value)
    is EntityRowText.Rich -> annotated
  }
}

@Composable
fun EntityRowMetaLine(
  text: String,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textHint,
  leading: (@Composable () -> Unit)? = null,
) {
  if (text.isBlank()) {
    return
  }

  Row(
    modifier = modifier.fillMaxWidth(),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(EntityRowDefaults.MetaSpacing),
  ) {
    if (leading != null) {
      leading()
    }

    Text(
      text = text,
      style = AppTheme.typography.caption,
      color = color,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
fun EntityRowParentMeta(
  folder: EntityParentMeta_folder,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textHint,
) {
  EntityRowMetaLine(
    text = formatFolderName(folder.name),
    modifier = modifier,
    color = color,
    leading = {
      EntityIcon(
        entity = folder.entity.entityIcon_entity,
        modifier = Modifier.size(EntityRowDefaults.MetaIconSize),
      )
    },
  )
}

@Composable
fun EntityRowSelectionControl(selected: Boolean, modifier: Modifier = Modifier) {
  Icon(
    icon = if (selected) Lucide.SquareCheck else Lucide.Square,
    modifier = modifier.size(EntityRowDefaults.IconSize),
    tint = if (selected) AppTheme.colors.textDefault else AppTheme.colors.textMuted,
  )
}

@Composable
fun EntityRowChevron(modifier: Modifier = Modifier) {
  Icon(
    icon = Lucide.ChevronRight,
    modifier = modifier.size(EntityRowDefaults.IconSize),
    tint = AppTheme.colors.textMuted,
  )
}
