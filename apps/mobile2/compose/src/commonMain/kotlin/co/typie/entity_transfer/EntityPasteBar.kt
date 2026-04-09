package co.typie.entity_transfer

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

internal val EntityPasteBarHeight = 48.dp

internal fun entityPasteBarToastBottomInset(baseInset: Dp): Dp = baseInset + EntityPasteBarHeight

@Composable
fun EntityPasteBar(
  bottomOffset: Dp,
  loading: Boolean,
  onClear: suspend () -> Unit,
  onPaste: suspend () -> Unit,
) {
  val enabled = !loading

  Box(
    modifier = Modifier
      .padding(horizontal = 16.dp)
      .padding(bottom = 16.dp + bottomOffset),
    contentAlignment = Alignment.Center,
  ) {
    Row(
      modifier = Modifier
        .alpha(if (enabled) 1f else 0.72f)
        .background(AppTheme.colors.brand, RoundedCornerShape(999.dp))
        .border(1.dp, AppTheme.colors.brand.copy(alpha = 0.14f), RoundedCornerShape(999.dp)),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      InteractionScope {
        Row(
          modifier = Modifier
            .clickable(enabled = enabled, onClick = onPaste)
            .pressScale(0.97f)
            .padding(start = 18.dp, top = 14.dp, end = 20.dp, bottom = 14.dp),
          horizontalArrangement = Arrangement.spacedBy(10.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = Lucide.ClipboardPaste,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textOnBrand,
          )

          Text(
            text = if (loading) "붙여넣는 중..." else "여기에 붙여넣기",
            style = AppTheme.typography.action,
            color = AppTheme.colors.textOnBrand,
          )
        }
      }

      Box(
        modifier = Modifier
          .width(1.dp)
          .height(18.dp)
          .background(AppTheme.colors.textOnBrand.copy(alpha = 0.22f)),
      )

      InteractionScope {
        Box(
          modifier = Modifier
            .size(EntityPasteBarHeight)
            .clickable(enabled = enabled, onClick = onClear)
            .pressScale(0.96f),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = Lucide.X,
            modifier = Modifier.size(18.dp),
            tint = AppTheme.colors.textOnBrand,
          )
        }
      }
    }
  }
}
