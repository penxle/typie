package co.typie

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import co.typie.auth.AuthService
import co.typie.auth.AuthState
import co.typie.platform.PurchaseActivityHolder
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {
  private val authService: AuthService by inject()

  override fun onCreate(savedInstanceState: Bundle?) {
    val splashScreen = installSplashScreen()
    splashScreen.setKeepOnScreenCondition { authService.state.value is AuthState.Initializing }
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
