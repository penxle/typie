package co.typie.ui.component.bottomsheet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.imeOrNavigationBarsPadding
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

private val BottomSheetEntityHeaderIconSize = 20.dp
private val BottomSheetEntityHeaderTitleGap = 12.dp
private val BottomSheetEntityHeaderTextLeft = BottomSheetEntityHeaderIconSize + BottomSheetEntityHeaderTitleGap

private val BottomSheetEntityMetadataTextStyle: TextStyle
  @Composable get() = AppTheme.typography.caption.copy(
    fontSize = 14.sp,
    lineHeight = 20.sp,
  )

@Composable
fun BottomSheetMenu(
  modifier: Modifier = Modifier,
  header: (@Composable ColumnScope.() -> Unit)? = null,
  showHeaderDivider: Boolean = true,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(
    modifier = modifier
      .fillMaxWidth()
      .imeOrNavigationBarsPadding()
      .padding(horizontal = 24.dp),
  ) {
    if (header != null) {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        content = header,
      )
      Spacer(Modifier.size(if (showHeaderDivider) 16.dp else 8.dp))
      if (showHeaderDivider) {
        BottomSheetMenuDivider()
        Spacer(Modifier.size(8.dp))
      }
    }

    Column(
      modifier = Modifier.fillMaxWidth(),
      content = content,
    )
  }
}

@Composable
fun BottomSheetMenuDivider(
  modifier: Modifier = Modifier,
  inset: Dp = 0.dp,
  color: Color = AppTheme.colors.borderDefault,
) {
  CardDivider(
    modifier = modifier,
    inset = inset,
    color = color,
  )
}

@Composable
fun BottomSheetMenuActionRow(
  icon: IconData,
  label: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  tint: Color? = null,
  trailingIcon: IconData? = null,
) {
  InteractionScope {
    Row(
      modifier = modifier
        .fillMaxWidth()
        .height(42.dp)
        .clickable(onClick = onClick)
        .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(
        icon = icon,
        modifier = Modifier.size(18.dp),
        tint = tint ?: AppTheme.colors.textPrimary,
      )

      Spacer(Modifier.width(12.dp))

      Text(
        text = label,
        modifier = Modifier.weight(1f),
        style = AppTheme.typography.action,
        color = tint ?: AppTheme.colors.textPrimary,
      )

      if (trailingIcon != null) {
        Icon(
          icon = trailingIcon,
          modifier = Modifier.size(14.dp),
          tint = tint ?: AppTheme.colors.textTertiary,
        )
      }
    }
  }
}

@Composable
fun BottomSheetEntityHeader(
  title: String,
  icon: IconData,
  modifier: Modifier = Modifier,
  iconTint: Color = AppTheme.colors.textPrimary,
  supportingContent: (@Composable ColumnScope.() -> Unit)? = null,
) {
  Column(
    modifier = modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(4.dp),
  ) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(BottomSheetEntityHeaderTitleGap),
    ) {
      Icon(
        icon = icon,
        modifier = Modifier.size(BottomSheetEntityHeaderIconSize),
        tint = iconTint,
      )

      Text(
        text = title,
        style = AppTheme.typography.title,
        modifier = Modifier.weight(1f),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }

    if (supportingContent != null) {
      Column(
        modifier = Modifier.padding(start = BottomSheetEntityHeaderTextLeft, end = 16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        content = supportingContent,
      )
    }
  }
}

@Composable
fun BottomSheetEntityBreadcrumb(
  segments: List<String>,
  modifier: Modifier = Modifier,
  textStyle: TextStyle = BottomSheetEntityMetadataTextStyle,
  color: Color = AppTheme.colors.textTertiary,
) {
  FlowRow(
    modifier = modifier,
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalArrangement = Arrangement.spacedBy(2.dp),
  ) {
    segments.forEachIndexed { index, segment ->
      if (index == 0) {
        Text(
          text = segment,
          style = textStyle,
          color = color,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          modifier = Modifier.widthIn(max = 220.dp),
        )
      } else {
        Row(
          horizontalArrangement = Arrangement.spacedBy(4.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(14.dp),
            tint = color,
          )

          Text(
            text = segment,
            style = textStyle,
            color = color,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.widthIn(max = 220.dp),
          )
        }
      }
    }
  }
}

@Composable
fun BottomSheetEntitySupportingText(
  text: String,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textMuted,
  textStyle: TextStyle = BottomSheetEntityMetadataTextStyle,
) {
  Text(
    text = text,
    modifier = modifier,
    style = textStyle,
    color = color,
  )
}
