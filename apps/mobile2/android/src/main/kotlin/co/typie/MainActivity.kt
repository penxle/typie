package co.typie

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.core.view.WindowCompat
import co.typie.domain.bootstrap.BootstrapService
import co.typie.domain.bootstrap.BootstrapState
import co.typie.domain.pushnotification.NotificationPermissionLauncher

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    NotificationPermissionLauncher.register(this)

    val splashScreen = installSplashScreen()
    splashScreen.setKeepOnScreenCondition { BootstrapService.state !is BootstrapState.Ready }

    enableEdgeToEdge()
    WindowCompat.setDecorFitsSystemWindows(window, false)

    super.onCreate(savedInstanceState)

    setContent { App() }
  }
}

@Preview
@Composable
fun AppAndroidPreview() {
  App()
}
