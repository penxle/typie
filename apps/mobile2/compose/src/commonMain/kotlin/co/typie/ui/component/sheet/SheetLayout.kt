package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.ime
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll

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
  padding: SheetPadding = SheetPadding(),
  verticalSpacing: Dp = 12.dp,
  header: (@Composable ColumnScope.() -> Unit)? = null,
  footer: (@Composable ColumnScope.() -> Unit)? = null,
  body: @Composable ColumnScope.() -> Unit,
) {
  val scrollState = rememberScrollState()
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val safeBottom = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()
  val bottomInset = maxOf(imeBottom, safeBottom)

  Column(
    modifier =
      modifier
        .fillMaxWidth()
        .then(if (fillHeight) Modifier.fillMaxHeight() else Modifier)
        .then(if (footer != null) Modifier.padding(bottom = bottomInset) else Modifier),
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
        if (footer == null && bottomInset > 0.dp) {
          Spacer(modifier = Modifier.height(bottomInset))
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
