package co.typie.ui.component.bottomsheet

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.imePadding
import co.typie.ext.pressScale
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
fun BottomSheetScaffold(
  title: String,
  modifier: Modifier = Modifier,
  leadingAction: (@Composable () -> Unit)? = null,
  trailingAction: (@Composable () -> Unit)? = null,
  content: @Composable ColumnScope.() -> Unit,
) {
  val scrollState = rememberScrollState()
  val leadingInset = if (leadingAction != null) TopBarDefaults.SlotWidth + 12.dp else 0.dp
  val trailingInset = if (trailingAction != null) TopBarDefaults.SlotWidth + 12.dp else 0.dp

  Column(
    modifier = modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp),
  ) {
    Box(
      modifier = Modifier
        .fillMaxWidth()
        .height(TopBarDefaults.SlotWidth),
    ) {
      Text(
        text = title,
        style = AppTheme.typography.title.copy(textAlign = TextAlign.Center),
        modifier = Modifier
          .align(Alignment.Center)
          .fillMaxWidth()
          .padding(start = leadingInset, end = trailingInset),
        overflow = TextOverflow.Ellipsis,
        maxLines = 1,
        color = AppTheme.colors.textPrimary,
      )

      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Box(
          modifier = Modifier.size(TopBarDefaults.SlotWidth),
          contentAlignment = Alignment.CenterStart,
        ) {
          leadingAction?.invoke()
        }

        Box(
          modifier = Modifier.size(TopBarDefaults.SlotWidth),
          contentAlignment = Alignment.CenterEnd,
        ) {
          trailingAction?.invoke()
        }
      }
    }

    Box(
      modifier = Modifier
        .fillMaxWidth()
        .weight(1f, fill = false)
        .padding(top = 8.dp)
        .imePadding()
        .verticalScroll(scrollState),
    ) {
      Column(
        modifier = Modifier
          .fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        content = content,
      )
    }
  }
}

@Composable
fun BottomSheetHeaderActionButton(
  icon: IconData,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  loading: Boolean = false,
  backgroundColor: Color? = null,
  borderColor: Color? = null,
  tint: Color? = null,
) {
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)
  val resolvedBackground = backgroundColor ?: TopBarDefaults.controlBackgroundColor()
  val resolvedBorderColor = borderColor ?: TopBarDefaults.controlBorderColor()
  val resolvedTint = tint ?: AppTheme.colors.textPrimary
  val shadowModifier = TopBarDefaults.controlShadowModifier(TopBarDefaults.ButtonShape)

  InteractionScope {
    Box(
      modifier = modifier
        .size(TopBarDefaults.ButtonSize)
        .alpha(alpha)
        .then(shadowModifier)
        .background(resolvedBackground, TopBarDefaults.ButtonShape)
        .border(1.dp, resolvedBorderColor, TopBarDefaults.ButtonShape)
        .clickable(enabled = enabled && !loading, onClick = onClick)
        .pressScale(0.94f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        BottomSheetHeaderActionSpinner(color = resolvedTint)
      } else {
        Icon(
          icon = icon,
          modifier = Modifier.size(TopBarDefaults.ButtonIconSize),
          tint = resolvedTint,
        )
      }
    }
  }
}

@Composable
fun BottomSheetHeaderTextAction(
  text: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  loading: Boolean = false,
  color: Color = AppTheme.colors.textPrimary,
  textStyle: TextStyle = AppTheme.typography.action,
) {
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)

  InteractionScope {
    Box(
      modifier = modifier
        .defaultMinSize(minWidth = TopBarDefaults.SlotWidth, minHeight = TopBarDefaults.SlotWidth)
        .alpha(alpha)
        .clickable(enabled = enabled && !loading, onClick = onClick)
        .pressScale(0.96f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        BottomSheetHeaderActionSpinner(color = color)
      } else {
        Text(
          text = text,
          style = textStyle,
          color = color,
        )
      }
    }
  }
}

@Composable
private fun BottomSheetHeaderActionSpinner(
  color: Color,
  modifier: Modifier = Modifier,
) {
  val transition = rememberInfiniteTransition()
  val rotation by transition.animateFloat(
    initialValue = 0f,
    targetValue = 360f,
    animationSpec = infiniteRepeatable(animation = tween(1000, easing = LinearEasing)),
  )

  Canvas(modifier.size(16.dp).then(modifier)) {
    drawArc(
      color = color,
      startAngle = rotation,
      sweepAngle = 220f,
      useCenter = false,
      style = androidx.compose.ui.graphics.drawscope.Stroke(
        width = 1.5.dp.toPx(),
        cap = androidx.compose.ui.graphics.StrokeCap.Round,
      ),
    )
  }
}
