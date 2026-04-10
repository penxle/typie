package co.typie

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import co.typie.platform.PurchaseActivityHolder
import co.typie.startup.AppStartupService
import co.typie.startup.AppStartupState

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    val splashScreen = installSplashScreen()
    AppStartupService.startAsync()
    splashScreen.setKeepOnScreenCondition { AppStartupService.state.value !is AppStartupState.Ready }

    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    setContent {
      App()
    }
  }

  override fun onResume() {
    super.onResume()
    PurchaseActivityHolder.attach(this)
  }

  override fun onPause() {
    PurchaseActivityHolder.detach(this)
    super.onPause()
  }
}

@Preview
@Composable
fun AppAndroidPreview() {
  App()
}
