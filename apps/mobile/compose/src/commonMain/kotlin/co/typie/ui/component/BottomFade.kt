package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsOrImePadding
import co.typie.ext.safeDrawingHorizontalPadding
import co.typie.ui.theme.AppTheme

context(boxScope: BoxScope)
@Composable
fun BottomFade(modifier: Modifier, content: @Composable ColumnScope.() -> Unit) {
  val fadeColor = AppTheme.colors.surfaceCanvas.copy(alpha = 0.7f)

  Column(
    modifier =
      with(boxScope) { Modifier.align(Alignment.BottomCenter) }
        .fillMaxWidth()
        .navigationBarsOrImePadding()
  ) {
    Spacer(
      Modifier.fillMaxWidth()
        .height(20.dp)
        .background(Brush.verticalGradient(colors = listOf(fadeColor.copy(alpha = 0f), fadeColor)))
    )

    Column(Modifier.fillMaxWidth().background(fadeColor)) {
      Column(modifier = Modifier.safeDrawingHorizontalPadding().then(modifier).fillMaxWidth()) {
        content()
      }
    }
  }
}
