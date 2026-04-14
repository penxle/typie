package co.typie.screen.settings.security_settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

private fun securityPasswordItemLabel(hasPassword: Boolean): String {
  return if (hasPassword) "비밀번호 변경" else "비밀번호 설정"
}

@Composable
fun SecuritySettingsScreen() {
  val nav = Nav.current
  val model = viewModel { SecuritySettingsViewModel() }
  val dialog = LocalDialog.current
  val scrollState = rememberScrollState()
  val hasPassword = model.query.data.me.hasPassword

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("보안", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("보안", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column {
          CardRow(onClick = { nav.navigate(Route.UpdatePassword) }) {
            SecurityRowContent(label = securityPasswordItemLabel(hasPassword))
          }

          CardDivider()

          CardRow(onClick = { nav.navigate(Route.SocialAccounts) }) {
            SecurityRowContent(label = "연결된 SNS 계정")
          }

          CardDivider()

          CardRow(onClick = { nav.navigate(Route.DeleteUser) }) {
            SecurityRowContent(label = "회원 탈퇴")
          }
        }
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun SecurityRowContent(label: String) {
  androidx.compose.foundation.layout.Row(
    modifier = Modifier.fillMaxWidth(),
    verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(10.dp),
  ) {
    Text(
      text = label,
      style = AppTheme.typography.label,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Icon(
      icon = Lucide.ChevronRight,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textTertiary,
    )
  }
}
