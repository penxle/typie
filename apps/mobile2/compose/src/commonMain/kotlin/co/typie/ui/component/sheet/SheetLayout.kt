package co.typie.ui.component.sheet

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
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.ime
import co.typie.ext.pressScale
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

sealed interface SheetInsetPolicy {
  data object Container : SheetInsetPolicy

  data object ContentTail : SheetInsetPolicy

  data object None : SheetInsetPolicy
}

@Immutable
data class SheetResolvedInset(val containerBottom: Dp = 0.dp, val contentTailBottom: Dp = 0.dp)

internal fun resolveSheetBottomInset(
  policy: SheetInsetPolicy,
  imeBottom: Dp,
  safeBottom: Dp,
): SheetResolvedInset {
  val bottom = maxOf(imeBottom, safeBottom)
  return when (policy) {
    SheetInsetPolicy.Container -> SheetResolvedInset(containerBottom = bottom)
    SheetInsetPolicy.ContentTail -> SheetResolvedInset(contentTailBottom = bottom)
    SheetInsetPolicy.None -> SheetResolvedInset()
  }
}

@Immutable
data class SheetPadding(
  val header: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
  val body: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
  val footer: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
) {
  companion object {
    val None =
      SheetPadding(
        header = PaddingValues(0.dp),
        body = PaddingValues(0.dp),
        footer = PaddingValues(0.dp),
      )
  }
}

@Composable
fun SheetLayout(
  modifier: Modifier = Modifier,
  fillHeight: Boolean = false,
  bodyScroll: Boolean = true,
  bodyInsetPolicy: SheetInsetPolicy = SheetInsetPolicy.ContentTail,
  padding: SheetPadding = SheetPadding(),
  verticalSpacing: Dp = 12.dp,
  header: (@Composable ColumnScope.() -> Unit)? = null,
  footer: (@Composable ColumnScope.() -> Unit)? = null,
  body: @Composable ColumnScope.() -> Unit,
) {
  val scrollState = rememberScrollState()
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val safeBottom = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()
  val resolvedInset =
    resolveSheetBottomInset(
      policy = bodyInsetPolicy,
      imeBottom = imeBottom,
      safeBottom = safeBottom,
    )

  Column(
    modifier =
      modifier
        .fillMaxWidth()
        .then(if (fillHeight) Modifier.fillMaxHeight() else Modifier)
        .padding(bottom = resolvedInset.containerBottom),
    verticalArrangement = Arrangement.spacedBy(verticalSpacing),
  ) {
    if (header != null) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(padding.header),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = header,
      )
    }

    Box(
      modifier =
        Modifier.fillMaxWidth()
          .weight(1f, fill = fillHeight)
          .then(if (bodyScroll) Modifier.verticalScroll(scrollState) else Modifier)
          .padding(padding.body)
    ) {
      Column(
        modifier =
          Modifier.fillMaxWidth()
            .then(
              if (fillHeight && !bodyScroll) {
                Modifier.fillMaxHeight()
              } else {
                Modifier
              }
            ),
        verticalArrangement = Arrangement.spacedBy(verticalSpacing),
      ) {
        body()
        if (resolvedInset.contentTailBottom > 0.dp) {
          Spacer(modifier = Modifier.height(resolvedInset.contentTailBottom))
        }
      }
    }

    if (footer != null) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(padding.footer),
        verticalArrangement = Arrangement.spacedBy(verticalSpacing),
        content = footer,
      )
    }
  }
}

@Composable
fun SheetHeader(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = modifier.fillMaxWidth(), content = content)
}

@Composable
fun SheetBody(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = modifier.fillMaxWidth(), content = content)
}

@Composable
fun SheetFooter(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = modifier.fillMaxWidth(), content = content)
}

@Composable
fun TitleHeader(
  title: String,
  modifier: Modifier = Modifier,
  titleStyle: TextStyle = AppTheme.typography.title.copy(textAlign = TextAlign.Center),
) {
  ActionHeader(title = title, modifier = modifier, titleStyle = titleStyle)
}

@Composable
fun ActionHeader(
  title: String,
  modifier: Modifier = Modifier,
  leading: (@Composable () -> Unit)? = null,
  trailing: (@Composable () -> Unit)? = null,
  titleStyle: TextStyle = AppTheme.typography.title.copy(textAlign = TextAlign.Center),
) {
  val leadingInset = if (leading != null) TopBarDefaults.SlotWidth + 12.dp else 0.dp
  val trailingInset = if (trailing != null) TopBarDefaults.SlotWidth + 12.dp else 0.dp
  val titleInset = maxOf(leadingInset, trailingInset)

  Box(modifier = modifier.fillMaxWidth().height(TopBarDefaults.SlotWidth)) {
    Text(
      text = title,
      style = titleStyle,
      modifier =
        Modifier.align(Alignment.Center)
          .fillMaxWidth()
          .padding(start = titleInset, end = titleInset),
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
        leading?.invoke()
      }

      Box(
        modifier = Modifier.size(TopBarDefaults.SlotWidth),
        contentAlignment = Alignment.CenterEnd,
      ) {
        trailing?.invoke()
      }
    }
  }
}

@Composable
fun HeaderActionButton(
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
      modifier =
        modifier
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
        HeaderActionSpinner(color = resolvedTint)
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
fun HeaderTextAction(
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
      modifier =
        modifier
          .defaultMinSize(minWidth = TopBarDefaults.SlotWidth, minHeight = TopBarDefaults.SlotWidth)
          .alpha(alpha)
          .clickable(enabled = enabled && !loading, onClick = onClick)
          .pressScale(0.96f),
      contentAlignment = Alignment.Center,
    ) {
      if (loading) {
        HeaderActionSpinner(color = color)
      } else {
        Text(text = text, style = textStyle, color = color)
      }
    }
  }
}

@Composable
private fun HeaderActionSpinner(color: Color, modifier: Modifier = Modifier) {
  val transition = rememberInfiniteTransition()
  val rotation by
    transition.animateFloat(
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
      style =
        androidx.compose.ui.graphics.drawscope.Stroke(
          width = 1.5.dp.toPx(),
          cap = androidx.compose.ui.graphics.StrokeCap.Round,
        ),
    )
  }
}

@Composable
fun TitleHeaderAction(
  title: String,
  actionText: String,
  onActionClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
) {
  ActionHeader(
    title = title,
    modifier = modifier,
    trailing = {
      HeaderTextAction(
        text = actionText,
        onClick = onActionClick,
        enabled = enabled,
        color = AppTheme.colors.brand,
        textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
      )
    },
  )
}
