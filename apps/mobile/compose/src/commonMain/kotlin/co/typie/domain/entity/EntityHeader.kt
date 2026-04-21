package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.graphql.fragment.EntityBreadcrumb_entity
import co.typie.graphql.fragment.EntityIcon_entity
import co.typie.ui.component.Text
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

object EntityHeaderDefaults {
  val IconSize = 20.dp
  val TitleGap = 12.dp
  val SupportingContentStartInset = IconSize + TitleGap
  val SupportingContentEndInset = 16.dp
}

internal val EntityMetadataTextStyle: TextStyle
  @Composable get() = AppTheme.typography.caption.copy(fontSize = 14.sp)

internal sealed interface EntityHeaderEntry

internal data class EntityHeaderBreadcrumbEntry(
  val entity: EntityBreadcrumb_entity,
  val modifier: Modifier,
  val layout: EntityBreadcrumbLayout,
  val color: Color?,
  val loading: Boolean,
) : EntityHeaderEntry

internal data class EntityHeaderSupportingTextEntry(
  val text: String?,
  val modifier: Modifier,
  val color: Color?,
  val loading: Boolean,
  val placeholderLength: IntRange,
) : EntityHeaderEntry

class EntityHeaderScope {
  @PublishedApi internal val entries = mutableListOf<EntityHeaderEntry>()

  fun breadcrumb(
    entity: EntityBreadcrumb_entity?,
    modifier: Modifier = Modifier,
    layout: EntityBreadcrumbLayout = EntityBreadcrumbLayout.FlowWrap,
    color: Color? = null,
    loading: Boolean = false,
  ) {
    if (entity == null) {
      return
    }

    entries.add(
      EntityHeaderBreadcrumbEntry(
        entity = entity,
        modifier = modifier,
        layout = layout,
        color = color,
        loading = loading,
      )
    )
  }

  fun supportingText(
    text: String?,
    modifier: Modifier = Modifier,
    color: Color? = null,
    loading: Boolean = false,
    placeholderLength: IntRange = 4..8,
  ) {
    if (text.isNullOrBlank() && !loading) {
      return
    }

    entries.add(EntityHeaderSupportingTextEntry(text, modifier, color, loading, placeholderLength))
  }
}

@Composable
fun EntityHeaderLayout(
  modifier: Modifier = Modifier,
  topContentModifier: Modifier = Modifier.fillMaxWidth(),
  supportingContentPadding: PaddingValues =
    PaddingValues(
      start = EntityHeaderDefaults.SupportingContentStartInset,
      end = EntityHeaderDefaults.SupportingContentEndInset,
    ),
  topContent: @Composable BoxScope.() -> Unit,
  supportingContent: (@Composable ColumnScope.() -> Unit)? = null,
) {
  Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
    Box(modifier = topContentModifier, content = topContent)

    if (supportingContent != null) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(supportingContentPadding),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        content = supportingContent,
      )
    }
  }
}

private fun buildEntityHeaderEntries(
  content: EntityHeaderScope.() -> Unit
): List<EntityHeaderEntry> {
  return EntityHeaderScope().apply(content).entries
}

@Composable
private fun RenderEntityHeaderEntries(entries: List<EntityHeaderEntry>) {
  entries.forEach { entry ->
    when (entry) {
      is EntityHeaderBreadcrumbEntry -> {
        Skeleton(enabled = entry.loading) {
          EntityBreadcrumb(
            entity = entry.entity,
            modifier = entry.modifier,
            layout = entry.layout,
            color = entry.color ?: AppTheme.colors.textMuted,
          )
        }
      }

      is EntityHeaderSupportingTextEntry -> {
        Skeleton(enabled = entry.loading) {
          EntitySupportingText(
            text = entry.text?.takeIf(String::isNotBlank) ?: Skeleton.text(entry.placeholderLength),
            modifier = entry.modifier,
            color = entry.color ?: AppTheme.colors.textHint,
          )
        }
      }
    }
  }
}

@Composable
fun EntityHeader(
  modifier: Modifier = Modifier,
  topContentModifier: Modifier = Modifier.fillMaxWidth(),
  supportingContentPadding: PaddingValues =
    PaddingValues(
      start = EntityHeaderDefaults.SupportingContentStartInset,
      end = EntityHeaderDefaults.SupportingContentEndInset,
    ),
  topContent: @Composable BoxScope.() -> Unit,
  content: EntityHeaderScope.() -> Unit = {},
) {
  val entries = buildEntityHeaderEntries(content)
  val supportingContent: (@Composable ColumnScope.() -> Unit)? =
    if (entries.isEmpty()) null else ({ RenderEntityHeaderEntries(entries) })

  EntityHeaderLayout(
    modifier = modifier,
    topContentModifier = topContentModifier,
    supportingContentPadding = supportingContentPadding,
    topContent = topContent,
    supportingContent = supportingContent,
  )
}

@Composable
private fun EntityHeaderTitleRow(
  title: String,
  icon: @Composable () -> Unit,
  trailing: (@Composable () -> Unit)?,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(EntityHeaderDefaults.TitleGap),
  ) {
    icon()

    Text(
      text = title,
      style = AppTheme.typography.title,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    if (trailing != null) {
      Box(contentAlignment = Alignment.CenterEnd) { trailing() }
    }
  }
}

@Composable
fun EntityHeader(
  title: String,
  entityIcon: EntityIcon_entity,
  modifier: Modifier = Modifier,
  trailing: (@Composable () -> Unit)? = null,
  content: EntityHeaderScope.() -> Unit = {},
) {
  val entries = buildEntityHeaderEntries(content)
  val supportingContent: (@Composable ColumnScope.() -> Unit)? =
    if (entries.isEmpty()) null else ({ RenderEntityHeaderEntries(entries) })

  EntityHeaderLayout(
    modifier = modifier,
    topContentModifier = Modifier.fillMaxWidth(),
    supportingContent = supportingContent,
    topContent = {
      EntityHeaderTitleRow(
        title = title,
        icon = {
          EntityIcon(entity = entityIcon, modifier = Modifier.size(EntityHeaderDefaults.IconSize))
        },
        trailing = trailing,
      )
    },
  )
}

@Composable
fun EntitySupportingText(
  text: String,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textHint,
) {
  Text(text = text, modifier = modifier, style = EntityMetadataTextStyle, color = color)
}
