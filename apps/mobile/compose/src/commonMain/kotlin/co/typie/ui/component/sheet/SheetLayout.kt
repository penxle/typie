package co.typie.ui.component.sheet

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.navigationBars
import co.typie.ext.thenIf
import co.typie.ext.verticalScroll
import co.typie.ui.component.SmootherstepEasing
import co.typie.ui.component.typieProgressiveBlur
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeProgressive
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState

private const val SheetOverlayHeaderFadeSamples = 48
private val SheetOverlayHeaderBlurRadius = 4.dp
private val SheetOverlayHeaderFadeHeight = 12.dp
private const val SheetOverlayHeaderTintOpacity = 0.84f

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
  handle: Boolean = true,
  handleModifier: Modifier = Modifier,
  includeBottomInset: Boolean = true,
  overlayHeader: Boolean = false,
  padding: SheetPadding = SheetPadding(),
  verticalSpacing: Dp = 12.dp,
  backgroundColor: Color = AppTheme.colors.surfaceCanvas,
  headerBackgroundColor: Color = backgroundColor,
  header: (@Composable ColumnScope.() -> Unit)? = null,
  footer: (@Composable ColumnScope.() -> Unit)? = null,
  body: @Composable ColumnScope.() -> Unit,
) {
  val scrollState = rememberScrollState()
  val bottomInsets =
    if (includeBottomInset) WindowInsets.navigationBars.union(WindowInsets.ime) else null

  val layoutModifier = modifier.fillMaxWidth().thenIf(fillHeight) { fillMaxHeight() }

  @Composable
  fun SheetBody(modifier: Modifier, includeTopSpacing: Boolean) {
    Column(modifier = modifier) {
      if (includeTopSpacing) Spacer(Modifier.height(verticalSpacing))

      Box(
        modifier =
          Modifier.fillMaxWidth()
            .weight(1f, fill = fillHeight)
            .thenIf(bodyScroll) { verticalScroll(scrollState) }
            .padding(padding.body)
      ) {
        Column(
          modifier = Modifier.fillMaxWidth().thenIf(fillHeight && !bodyScroll) { fillMaxHeight() },
          verticalArrangement = Arrangement.spacedBy(verticalSpacing),
          content = body,
        )
      }

      if (footer != null || bottomInsets != null) {
        Spacer(Modifier.height(verticalSpacing))

        if (footer != null) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(padding.footer),
            verticalArrangement = Arrangement.spacedBy(verticalSpacing),
            content = footer,
          )
        }

        if (bottomInsets != null) {
          Spacer(Modifier.windowInsetsBottomHeight(bottomInsets))
        }
      }
    }
  }

  if (overlayHeader) {
    val headerHazeState = rememberHazeState()
    val headerBackgroundColorState = rememberUpdatedState(headerBackgroundColor)
    val headerProgressiveBrush = remember { sheetOverlayHeaderProgressiveBrush(Color.Black) }
    val headerBlurModifier =
      typieProgressiveBlur(
        hazeState = headerHazeState,
        radius = SheetOverlayHeaderBlurRadius,
        progressiveBrush = headerProgressiveBrush,
        fallbackProgressive =
          HazeProgressive.verticalGradient(
            easing = SmootherstepEasing,
            startIntensity = 1f,
            endIntensity = 0f,
          ),
        backdropColor = { headerBackgroundColorState.value },
      )
    val tintBrush =
      remember(headerBackgroundColor) {
        sheetOverlayHeaderProgressiveBrush(
          headerBackgroundColor.copy(alpha = SheetOverlayHeaderTintOpacity)
        )
      }

    Box(modifier = layoutModifier.background(backgroundColor)) {
      SheetBody(
        modifier =
          Modifier.fillMaxWidth()
            .thenIf(fillHeight) { fillMaxHeight() }
            .background(backgroundColor)
            .hazeSource(headerHazeState),
        includeTopSpacing = false,
      )

      Column(
        modifier =
          Modifier.fillMaxWidth()
            .then(headerBlurModifier)
            .drawBehind { drawRect(tintBrush) }
            .pointerInput(Unit) {}
      ) {
        if (handle) SheetHandle(modifier = handleModifier)

        if (header != null) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(padding.header),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            content = header,
          )
        }

        Spacer(Modifier.height(SheetOverlayHeaderFadeHeight))
      }
    }
    return
  }

  Column(modifier = layoutModifier) {
    if (handle) SheetHandle(modifier = handleModifier.background(headerBackgroundColor))

    if (header != null) {
      Column(
        modifier =
          Modifier.fillMaxWidth().background(headerBackgroundColor).padding(padding.header),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = header,
      )
    }

    SheetBody(modifier = Modifier.background(backgroundColor), includeTopSpacing = true)
  }
}

private fun sheetOverlayHeaderProgressiveBrush(color: Color): Brush {
  val stops =
    Array(SheetOverlayHeaderFadeSamples + 1) { index ->
      val t = index / SheetOverlayHeaderFadeSamples.toFloat()
      val alpha = color.alpha * (1f - SmootherstepEasing.transform(t))
      t to color.copy(alpha = alpha)
    }

  return Brush.verticalGradient(colorStops = stops)
}

private val HandleTopPadding = 8.dp
private val HandleIndicatorHeight = 4.dp
private val HandleBottomPadding = 8.dp
private val HandleWidth = 36.dp
internal val SheetHandleContainerHeight =
  HandleTopPadding + HandleIndicatorHeight + HandleBottomPadding

@Composable
private fun SheetHandle(modifier: Modifier) {
  Box(
    modifier = modifier.fillMaxWidth().height(SheetHandleContainerHeight),
    contentAlignment = Alignment.Center,
  ) {
    Box(
      modifier =
        Modifier.size(width = HandleWidth, height = HandleIndicatorHeight)
          .clip(AppShapes.rounded(AppShapes.sm))
          .background(AppTheme.colors.borderHairline)
    )
  }
}
