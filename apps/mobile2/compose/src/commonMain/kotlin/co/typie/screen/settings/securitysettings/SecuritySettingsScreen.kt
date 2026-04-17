package co.typie.screen.settings.securitysettings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.settings.SettingsCardRow
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun SecuritySettingsScreen() {
  val model = viewModel { SecuritySettingsViewModel() }
  val scrollState = rememberScrollState()
  val nav = Nav.current

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("보안", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("보안", style = AppTheme.typography.display)

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column {
          SettingsCardRow(
            label = if (model.query.data.me.hasPassword) "비밀번호 변경" else "비밀번호 설정",
            onClick = { nav.navigate(Route.UpdatePassword) },
          )

          CardDivider()

          SettingsCardRow(label = "연결된 SNS 계정", onClick = { nav.navigate(Route.SocialAccounts) })

          CardDivider()

          SettingsCardRow(label = "회원 탈퇴", onClick = { nav.navigate(Route.DeleteUser) })
        }
      }
    }
  }
}
