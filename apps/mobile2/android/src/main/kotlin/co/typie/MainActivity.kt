package co.typie

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import co.typie.domain.bootstrap.BootstrapService
import co.typie.domain.bootstrap.BootstrapState

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    val splashScreen = installSplashScreen()
    splashScreen.setKeepOnScreenCondition { BootstrapService.state !is BootstrapState.Ready }

    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    setContent { App() }
  }
}

@Preview
@Composable
fun AppAndroidPreview() {
  App()
}
