package co.typie.screen.settings.socialaccounts

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
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.separated
import co.typie.ext.thenIfNotNull
import co.typie.ext.verticalScroll
import co.typie.generated.resources.Res
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun SocialAccountsScreen() {
  val model = viewModel { SocialAccountsViewModel() }
  val scrollState = rememberScrollState()

  ProvideTopBar(center = { Text("연결된 SNS 계정", style = AppTheme.typography.title) })

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Spacer(Modifier.height(4.dp))

      SectionTitle("연결된 계정")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        if (model.query.data.me.singleSignOns.isEmpty()) {
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

            Text(
              "연결된 SNS 계정이 없어요",
              style = AppTheme.typography.label,
              color = AppTheme.colors.textTertiary,
            )
          }
        } else {
          Column {
            model.query.data.me.singleSignOns.separated(separator = { CardDivider() }) {
              SocialAccountRow(provider = it.provider, email = it.email)
            }
          }
        }
      }
    }
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
      Text(provider.toProviderName(), style = AppTheme.typography.action)
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
        .thenIfNotNull(border) { border(1.dp, it, AppShapes.rounded(AppShapes.sm)) }
        .background(background, AppShapes.rounded(AppShapes.sm)),
    contentAlignment = Alignment.Center,
    content = content,
  )
}

private fun SingleSignOnProvider.toProviderName() =
  when (this) {
    SingleSignOnProvider.UNKNOWN__ -> rawValue
    else -> this.name.lowercase().replaceFirstChar { it.uppercase() }
  }
