package co.typie.screen.system.splash

import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import co.typie.generated.resources.Res
import co.typie.ui.component.Img
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun SplashScreen() {
  val model = viewModel { SplashViewModel() }
  val isDark = isSystemInDarkTheme()
  val backgroundColor = if (isDark) Color.Black else Color.White
  val iconTint = if (isDark) Color.White else Color(0xFFFAAD00)

  Box(
    modifier = Modifier.fillMaxSize().background(backgroundColor),
    contentAlignment = Alignment.Center,
  ) {
    Img(
      url = Res.getUri("files/logos/full.svg"),
      modifier = Modifier.size(64.dp),
      contentScale = ContentScale.Fit,
      color = iconTint,
    )
  }
}
