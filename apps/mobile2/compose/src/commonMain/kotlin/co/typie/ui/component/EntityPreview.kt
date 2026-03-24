package co.typie.ui.component

import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import co.typie.ext.toDp
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
fun EntityPreview(url: String, modifier: Modifier = Modifier, placeholderColor: Color? = null) {
  val density = LocalDensity.current
  val theme = AppTheme.themeMode.name.lowercase()

  BoxWithConstraints(modifier = modifier) {
    val width = (constraints.maxWidth * density.density).roundToInt()

    Img(
      url = "${url}&w=$width&theme=$theme",
      modifier = Modifier
        .fillMaxWidth()
        .height(constraints.maxWidth.toDp(density) * 4 / 3),
      placeholderColor = placeholderColor,
    )
  }
}
