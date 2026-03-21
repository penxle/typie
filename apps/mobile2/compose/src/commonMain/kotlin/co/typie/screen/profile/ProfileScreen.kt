package co.typie.screen.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.auth.AuthService
import co.typie.ext.clickable
import co.typie.graphql.GraphQLContent
import co.typie.graphql.ProfileScreen_Query
import co.typie.graphql.rememberQuery
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ThemeMode
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ProfileScreen() {
  val authService = koinInject<AuthService>()
  val toast = koinInject<Toast>()
  val scope = rememberCoroutineScope()
  val query = rememberQuery(ProfileScreen_Query())

  Screen { _ ->
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
        Text(data.me.name, style = AppTheme.typography.title)
        Text(data.me.email, style = AppTheme.typography.caption, color = AppTheme.colors.textFaint)
        val themeMode = LocalThemeMode.current
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
          ThemeMode.entries.forEach { mode ->
            val isSelected = themeMode.value == mode
            Text(
              mode.name,
              style = AppTheme.typography.action,
              color = if (isSelected) AppTheme.colors.textBrand else AppTheme.colors.textMuted,
              modifier = Modifier
                .clip(RoundedCornerShape(8.dp))
                .clickable {
                  themeMode.value = mode
                  toast.show(ToastType.Success, "Theme mode changed to ${mode.name}")
                },
            )
          }
        }
        Text(
          "로그아웃",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textDanger,
          modifier = Modifier.clickable { scope.launch { authService.logout() } },
        )
      }
    }
  }
}
