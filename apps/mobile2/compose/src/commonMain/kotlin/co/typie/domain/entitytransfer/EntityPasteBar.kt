package co.typie.domain.entitytransfer

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.EntityBottomOverlayDefaults
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private val EntityPasteBarShape = AppShapes.rounded(AppShapes.full)

internal fun entityPasteBarToastBottomInset(baseInset: Dp): Dp =
  baseInset + EntityBottomOverlayDefaults.BarHeight

@Composable
fun EntityPasteBar(
  loading: Boolean,
  modifier: Modifier = Modifier,
  onClear: suspend () -> Unit,
  onPaste: suspend () -> Unit,
) {
  val enabled = !loading
  val colors = AppTheme.colors

  Box(modifier = modifier, contentAlignment = Alignment.Center) {
    Row(
      modifier =
        Modifier.graphicsLayer { alpha = if (enabled) 1f else 0.72f }
          .dropShadow(EntityPasteBarShape) {
            color = colors.shadowAmbient
            radius = 3f
          }
          .dropShadow(EntityPasteBarShape) {
            color = colors.shadow
            offset = Offset(0f, 4f)
            radius = 16f
          }
          .background(AppTheme.colors.brand, EntityPasteBarShape)
          .border(1.dp, AppTheme.colors.brand.copy(alpha = 0.14f), EntityPasteBarShape),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      InteractionScope {
        Box(
          modifier =
            Modifier.height(EntityBottomOverlayDefaults.BarHeight)
              .clickable(enabled = enabled, onClick = onPaste)
              .pressScale(0.97f)
              .padding(start = 18.dp, end = 20.dp)
              .wrapContentWidth(),
          contentAlignment = Alignment.Center,
        ) {
          Row(
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
      }

      Box(
        modifier =
          Modifier.width(1.dp)
            .height(18.dp)
            .background(AppTheme.colors.textOnBrand.copy(alpha = 0.22f))
      )

      InteractionScope {
        Box(
          modifier =
            Modifier.size(EntityBottomOverlayDefaults.BarHeight)
              .clickable(enabled = enabled, onClick = onClear)
              .pressScale(0.96f),
          contentAlignment = Alignment.Center,
        ) {
          Icon(icon = Lucide.X, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textOnBrand)
        }
      }
    }
  }
}
