package co.typie.screen.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.auth.AuthService
import co.typie.ui.clickable
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ProfileScreen() {
  val authService = koinInject<AuthService>()
  val scope = rememberCoroutineScope()

  Screen {
    Column(
      Modifier.fillMaxSize(),
      verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      Text("Profile", style = TextStyle(fontSize = 20.sp))
      Text(
        "로그아웃",
        style = TextStyle(fontSize = 15.sp, color = AppTheme.colors.textDanger),
        modifier = Modifier.clickable { scope.launch { authService.logout() } },
      )
    }
  }
}
