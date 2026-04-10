package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.theme.AppTheme

object CardDefaults {
  val Shape: Shape = RoundedCornerShape(12.dp)
  val RowPadding: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 16.dp)
  val TilePadding: PaddingValues = PaddingValues(16.dp)
  val TileMinHeight: Dp = 108.dp
  val DividerInset: Dp = 16.dp
}

@Composable
fun CardSurface(
  modifier: Modifier = Modifier,
  shape: Shape = CardDefaults.Shape,
  color: Color = AppTheme.colors.surfaceDefault,
  clipContent: Boolean = true,
  content: @Composable BoxScope.() -> Unit,
) {
  Box(
    modifier =
      if (clipContent) {
        modifier.clip(shape).background(color, shape)
      } else {
        modifier.background(color, shape)
      },
    content = content,
  )
}

@Composable
fun CardActionTile(
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  minHeight: Dp = CardDefaults.TileMinHeight,
  color: Color = AppTheme.colors.surfaceDefault,
  contentPadding: PaddingValues = CardDefaults.TilePadding,
  content: @Composable ColumnScope.() -> Unit,
) {
  InteractionScope {
    CardSurface(modifier = modifier.clickable(onClick), color = color) {
      Column(
        modifier =
          Modifier.fillMaxWidth().heightIn(min = minHeight).padding(contentPadding).pressScale(),
        verticalArrangement = Arrangement.SpaceBetween,
        content = content,
      )
    }
  }
}

@Composable
fun CardRow(
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  contentPadding: PaddingValues = CardDefaults.RowPadding,
  spacing: Dp = 10.dp,
  content: @Composable RowScope.() -> Unit,
) {
  InteractionScope {
    Row(
      modifier = modifier.fillMaxWidth().clickable(onClick).padding(contentPadding).pressScale(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(spacing),
      content = content,
    )
  }
}

@Composable
fun CardDivider(
  modifier: Modifier = Modifier,
  inset: Dp = CardDefaults.DividerInset,
  color: Color = AppTheme.colors.borderSubtle,
) {
  Box(modifier = modifier.fillMaxWidth().height(1.dp).padding(horizontal = inset).background(color))
}
