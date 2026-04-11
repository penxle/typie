package co.typie.ui.component

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
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
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

object EntityHeaderDefaults {
  val IconSize = 20.dp
  val TitleGap = 12.dp
  val SupportingContentStartInset = IconSize + TitleGap
  val SupportingContentEndInset = 16.dp
}

enum class EntityBreadcrumbLayout {
  FlowWrap,
  SingleLineEllipsis,
}

private val EntityMetadataTextStyle: TextStyle
  @Composable get() = AppTheme.typography.caption.copy(fontSize = 14.sp)

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

@Composable
fun EntityHeader(
  title: String,
  icon: IconData,
  modifier: Modifier = Modifier,
  iconTint: Color = AppTheme.colors.textPrimary,
  trailing: (@Composable () -> Unit)? = null,
  supportingContent: (@Composable ColumnScope.() -> Unit)? = null,
) {
  EntityHeader(
    modifier = modifier,
    topContentModifier = Modifier.fillMaxWidth(),
    supportingContent = supportingContent,
    topContent = {
      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(EntityHeaderDefaults.TitleGap),
      ) {
        Icon(icon = icon, modifier = Modifier.size(EntityHeaderDefaults.IconSize), tint = iconTint)

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
    },
  )
}

@Composable
fun EntityBreadcrumb(
  segments: List<String>,
  modifier: Modifier = Modifier,
  layout: EntityBreadcrumbLayout = EntityBreadcrumbLayout.FlowWrap,
  color: Color = AppTheme.colors.textTertiary,
) {
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
              verticalAlignment = Alignment.CenterVertically,
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
        verticalAlignment = Alignment.CenterVertically,
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
              verticalAlignment = Alignment.CenterVertically,
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

@Composable
fun EntitySupportingText(
  text: String,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textMuted,
) {
  Text(text = text, modifier = modifier, style = EntityMetadataTextStyle, color = color)
}
