package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.graphql.fragment.EntityBreadcrumb_entity
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

enum class EntityBreadcrumbLayout {
  FlowWrap,
  SingleLineEllipsis,
}

fun EntityBreadcrumb_entity.breadcrumbSegments(): List<String> {
  return buildList {
    site.name.takeIf { it.isNotBlank() }?.let(::add)
    addAll(ancestors.mapNotNull { it.node.onFolder?.name })
  }
}

@Composable
fun EntityBreadcrumb(
  entity: EntityBreadcrumb_entity,
  modifier: Modifier = Modifier,
  layout: EntityBreadcrumbLayout = EntityBreadcrumbLayout.FlowWrap,
  color: Color = AppTheme.colors.textMuted,
) {
  val segments = entity.breadcrumbSegments()

  when (layout) {
    EntityBreadcrumbLayout.FlowWrap -> {
      FlowRow(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
      ) {
        segments.forEachIndexed { index, segment ->
          if (index == 0) {
            Text(
              text = segment,
              style = EntityMetadataTextStyle,
              color = color,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Row(
              horizontalArrangement = Arrangement.spacedBy(4.dp),
              verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
            ) {
              Icon(icon = Lucide.ChevronRight, modifier = Modifier.size(14.dp), tint = color)

              Text(
                text = segment,
                style = EntityMetadataTextStyle,
                color = color,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
          }
        }
      }
    }

    EntityBreadcrumbLayout.SingleLineEllipsis -> {
      Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
      ) {
        segments.forEachIndexed { index, segment ->
          if (index == 0) {
            Text(
              text = segment,
              modifier = Modifier.weight(1f, fill = false),
              style = EntityMetadataTextStyle,
              color = color,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          } else {
            Row(
              modifier = Modifier.weight(1f, fill = false),
              horizontalArrangement = Arrangement.spacedBy(4.dp),
              verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
            ) {
              Icon(icon = Lucide.ChevronRight, modifier = Modifier.size(14.dp), tint = color)

              Text(
                text = segment,
                modifier = Modifier.weight(1f, fill = false),
                style = EntityMetadataTextStyle,
                color = color,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
          }
        }
      }
    }
  }
}
