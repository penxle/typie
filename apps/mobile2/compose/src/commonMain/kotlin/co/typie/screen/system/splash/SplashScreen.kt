package co.typie.screen.system.splash

import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import co.typie.generated.resources.Res
import co.typie.ui.component.Img
import co.typie.ui.theme.AppColor

@Composable
fun SplashScreen() {
  val (backgroundColor, iconColor) =
    when (isSystemInDarkTheme()) {
      true -> AppColor.light.gray.s950 to AppColor.white
      false -> AppColor.white to AppColor.light.gray.s950
    }

  Box(
    modifier = Modifier.fillMaxSize().background(backgroundColor),
    contentAlignment = Alignment.Center,
  ) {
    Img(
      url = Res.getUri("files/logos/full.svg"),
      modifier = Modifier.size(64.dp),
      contentScale = ContentScale.Fit,
      color = iconColor,
    )
  }
}
