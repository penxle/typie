package co.typie

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import co.typie.bootstrap.BootstrapService
import co.typie.bootstrap.BootstrapState
import co.typie.platform.PurchaseActivityHolder

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    val splashScreen = installSplashScreen()
    splashScreen.setKeepOnScreenCondition { BootstrapService.state.value !is BootstrapState.Ready }

    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    setContent { App() }
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
