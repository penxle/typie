package co.typie.ui.component

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
internal fun ExpandableMetric(
  icon: IconData,
  label: String,
  value: String,
  expanded: Boolean,
  onToggle: () -> Unit,
  modifier: Modifier = Modifier,
  valueColor: Color = AppTheme.colors.textMuted,
  valueIcon: IconData? = null,
  content: @Composable () -> Unit,
) {
  InteractionScope {
    Column(
      modifier =
        modifier.fillMaxWidth().clickable { onToggle() }.pressScale().padding(vertical = 16.dp)
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Row(
          modifier = Modifier.weight(1f),
          horizontalArrangement = Arrangement.spacedBy(4.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(icon = icon, modifier = Modifier.size(14.dp), tint = AppTheme.colors.textHint)
          Text(
            text = label,
            style = AppTheme.typography.action,
            color = AppTheme.colors.textDefault,
          )
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(15.dp).rotate(if (expanded) 90f else 0f),
            tint = AppTheme.colors.textHint,
          )
        }

        AnimatedVisibility(
          visible = !expanded,
          enter =
            fadeIn(
              animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
            ),
          exit =
            fadeOut(
              animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
            ),
        ) {
          Row(
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            if (valueIcon != null) {
              Icon(icon = valueIcon, modifier = Modifier.size(14.dp), tint = valueColor)
            }

            Text(
              text = value,
              style = AppTheme.typography.action.copy(fontWeight = FontWeight.W500),
              color = valueColor,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          }
        }
      }

      AnimatedVisibility(
        visible = expanded,
        enter =
          fadeIn(
            animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
          ) +
            expandVertically(
              animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic),
              expandFrom = Alignment.Top,
            ),
        exit =
          fadeOut(
            animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic)
          ) +
            shrinkVertically(
              animationSpec = tween(ExpandableMetricAnimationDurationMillis, easing = EaseOutCubic),
              shrinkTowards = Alignment.Top,
            ),
      ) {
        Column(modifier = Modifier.fillMaxWidth().padding(top = 10.dp)) { content() }
      }
    }
  }
}

private const val ExpandableMetricAnimationDurationMillis = 220
