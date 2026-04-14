package co.typie.screen.settings.social_accounts

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.generated.resources.Res
import co.typie.graphql.Apollo
import co.typie.graphql.QueryState
import co.typie.graphql.SocialAccountsScreen_Query
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

internal fun socialAccountProviderName(provider: SingleSignOnProvider): String {
  return when (provider) {
    SingleSignOnProvider.GOOGLE -> "Google"
    SingleSignOnProvider.NAVER -> "Naver"
    SingleSignOnProvider.KAKAO -> "Kakao"
    SingleSignOnProvider.APPLE -> "Apple"
    SingleSignOnProvider.UNKNOWN__ -> provider.rawValue
  }
}

@Composable
fun SocialAccountsScreen() {
  val nav = Nav.current
  val model = viewModel { SocialAccountsViewModel() }
  val dialog = LocalDialog.current
  val scrollState = rememberScrollState()
  val singleSignOns = model.query.data?.me?.singleSignOns.orEmpty()

  ProvideTopBar(center = { Text("연결된 SNS 계정", style = AppTheme.typography.title) })

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Spacer(Modifier.height(4.dp))

      SectionTitle("연결된 계정")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        if (singleSignOns.isEmpty()) {
          EmptySocialAccountsState()
        } else {
          Column {
            singleSignOns.forEachIndexed { index, singleSignOn ->
              SocialAccountRow(provider = singleSignOn.provider, email = singleSignOn.email)

              if (index < singleSignOns.lastIndex) {
                CardDivider()
              }
            }
          }
        }
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun EmptySocialAccountsState() {
  Column(
    modifier = Modifier.fillMaxWidth().height(220.dp).padding(horizontal = 16.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.Center,
  ) {
    Icon(
      icon = Lucide.UserRoundX,
      modifier = Modifier.size(48.dp),
      tint = AppTheme.colors.textTertiary,
    )
    Spacer(Modifier.height(12.dp))
    Text("연결된 SNS 계정이 없어요", style = AppTheme.typography.label, color = AppTheme.colors.textTertiary)
  }
}

@Composable
private fun SocialAccountRow(provider: SingleSignOnProvider, email: String) {
  Row(
    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    ProviderIcon(provider = provider)

    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
      Text(socialAccountProviderName(provider), style = AppTheme.typography.action)
      Text(email, style = AppTheme.typography.body, color = AppTheme.colors.textTertiary)
    }
  }
}

@Composable
private fun ProviderIcon(provider: SingleSignOnProvider) {
  when (provider) {
    SingleSignOnProvider.GOOGLE -> {
      Img(url = Res.getUri("files/brands/google.svg"), modifier = Modifier.size(28.dp))
    }

    SingleSignOnProvider.NAVER -> {
      ProviderIconBadge(background = Color(0xFF03C75A)) {
        Img(
          url = Res.getUri("files/brands/naver.svg"),
          modifier = Modifier.size(16.dp),
          color = Color.White,
        )
      }
    }

    SingleSignOnProvider.KAKAO -> {
      ProviderIconBadge(background = Color(0xFFFEE500)) {
        Img(
          url = Res.getUri("files/brands/kakao.svg"),
          modifier = Modifier.size(20.dp),
          color = Color(0xFF000000),
        )
      }
    }

    SingleSignOnProvider.APPLE -> {
      ProviderIconBadge(background = Color(0xFF000000)) {
        Img(
          url = Res.getUri("files/brands/apple.svg"),
          modifier = Modifier.size(16.dp),
          color = Color.White,
        )
      }
    }

    SingleSignOnProvider.UNKNOWN__ -> {
      ProviderIconBadge(
        background = AppTheme.colors.surfaceDefault,
        border = AppTheme.colors.borderDefault,
      ) {
        Icon(
          icon = Lucide.User,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textTertiary,
        )
      }
    }
  }
}

@Composable
private fun ProviderIconBadge(
  background: Color,
  border: Color? = null,
  content: @Composable BoxScope.() -> Unit,
) {
  Box(
    modifier =
      Modifier.size(28.dp)
        .then(
          if (border != null) Modifier.border(1.dp, border, RoundedCornerShape(6.dp)) else Modifier
        )
        .background(background, RoundedCornerShape(6.dp)),
    contentAlignment = Alignment.Center,
    content = content,
  )
}

internal class SocialAccountsViewModel : ViewModel() {

  val query = Apollo.watchQuery(scope = viewModelScope) { SocialAccountsScreen_Query() }
}
