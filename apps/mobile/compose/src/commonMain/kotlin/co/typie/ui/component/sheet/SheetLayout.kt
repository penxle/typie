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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.navigationBars
import co.typie.ext.thenIf
import co.typie.ext.verticalScroll
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

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

  Column(modifier = modifier.fillMaxWidth().thenIf(fillHeight) { fillMaxHeight() }) {
    if (handle) SheetHandle(modifier = handleModifier.background(headerBackgroundColor))

    if (header != null) {
      Column(
        modifier =
          Modifier.fillMaxWidth().background(headerBackgroundColor).padding(padding.header),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = header,
      )
    }

    Column(modifier = Modifier.background(backgroundColor)) {
      Spacer(Modifier.height(verticalSpacing))

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
        ) {
          body()
        }
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
}

private val HandleTopPadding = 8.dp
private val HandleHeight = 4.dp
private val HandleBottomPadding = 8.dp
private val HandleWidth = 36.dp

@Composable
private fun SheetHandle(modifier: Modifier) {
  Box(
    modifier =
      modifier.fillMaxWidth().height(HandleTopPadding + HandleHeight + HandleBottomPadding),
    contentAlignment = Alignment.Center,
  ) {
    Box(
      modifier =
        Modifier.size(width = HandleWidth, height = HandleHeight)
          .clip(AppShapes.rounded(AppShapes.sm))
          .background(AppTheme.colors.borderHairline)
    )
  }
}
