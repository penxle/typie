package co.typie.screen.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.auth.AuthService
import co.typie.graphql.GraphQLContent
import co.typie.graphql.ProfileScreen_Query
import co.typie.graphql.rememberQuery
import co.typie.ui.clickable
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ProfileScreen() {
  val authService = koinInject<AuthService>()
  val scope = rememberCoroutineScope()
  val query = rememberQuery(ProfileScreen_Query())

  Screen {
    GraphQLContent(query) { data ->
      Column(
        Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        Img(
          image = data.me.avatar.img_image,
          size = 64.dp,
          modifier = Modifier.clip(CircleShape),
        )
        Text(data.me.name, style = TextStyle(fontSize = 20.sp))
        Text(data.me.email, style = TextStyle(fontSize = 15.sp, color = AppTheme.colors.textFaint))
        Text(
          "로그아웃",
          style = TextStyle(fontSize = 15.sp, color = AppTheme.colors.textDanger),
          modifier = Modifier.clickable { scope.launch { authService.logout() } },
        )
      }
    }
  }
}
