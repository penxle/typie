package co.typie.screen.login

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import co.typie.auth.sso.activityContext
import co.typie.di.Platform
import co.typie.ext.clickable
import co.typie.ext.safeDrawingPadding
import co.typie.generated.resources.Res
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginScreen() {
  val nav = Nav.current
  val viewModel = koinViewModel<LoginViewModel>()
  val platform = koinInject<Platform>()
  val ctx = activityContext()

  ProvideTopBar(
    enabled = false
  )

  Screen { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .safeDrawingPadding()
    ) {
      Column(
        modifier = Modifier.weight(1f).fillMaxWidth(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        AsyncImage(
          model = Res.getUri("files/logos/full.svg"),
          contentDescription = null,
          modifier = Modifier.height(32.dp),
          contentScale = ContentScale.FillHeight,
          colorFilter = ColorFilter.tint(AppTheme.colors.textDefault),
        )
        Spacer(Modifier.height(24.dp))
        Text("작성, 정리, 공유까지.", style = AppTheme.typography.action)
        Spacer(Modifier.height(4.dp))
        Text("글쓰기의 모든 과정을", style = AppTheme.typography.action)
        Spacer(Modifier.height(4.dp))
        Text("타이피 하나로 해결해요.", style = AppTheme.typography.action)
      }

      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        SingleSignOnButton(
          text = "구글로 시작하기",
          svgPath = "files/brands/google.svg",
          foregroundColor = Color(0xFF000000),
          backgroundColor = Color(0xFFFFFFFF),
          borderColor = AppTheme.colors.borderDefault,
          onClick = { viewModel.loginWith(SingleSignOnProvider.GOOGLE, ctx) },
        )
        SingleSignOnButton(
          text = "카카오로 시작하기",
          svgPath = "files/brands/kakao.svg",
          iconTint = Color(0xFF000000),
          foregroundColor = Color(0xFF000000),
          backgroundColor = Color(0xFFFEE500),
          onClick = { viewModel.loginWith(SingleSignOnProvider.KAKAO, ctx) },
        )
        SingleSignOnButton(
          text = "네이버로 시작하기",
          svgPath = "files/brands/naver.svg",
          iconTint = Color(0xFFFFFFFF),
          foregroundColor = Color(0xFFFFFFFF),
          backgroundColor = Color(0xFF03C75A),
          onClick = { viewModel.loginWith(SingleSignOnProvider.NAVER, ctx) },
        )

        if (platform != Platform.Android) {
          SingleSignOnButton(
            text = "애플로 시작하기",
            svgPath = "files/brands/apple.svg",
            iconTint = Color(0xFFFFFFFF),
            foregroundColor = Color(0xFFFFFFFF),
            backgroundColor = Color(0xFF000000),
            onClick = { viewModel.loginWith(SingleSignOnProvider.APPLE, ctx) },
          )
        }

        Text(
          "이메일로 가입하셨나요?",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textSubtle,
          modifier = Modifier
            .padding(vertical = 8.dp)
            .clickable { nav.navigate(Route.LoginWithEmail) },
        )
      }
    }
  }
}

@Composable
private fun SingleSignOnButton(
  text: String,
  svgPath: String,
  foregroundColor: Color,
  backgroundColor: Color,
  borderColor: Color? = null,
  iconTint: Color? = null,
  onClick: () -> Unit = {},
) {
  val shape = RoundedCornerShape(16.dp)

  Box(
    modifier = Modifier
      .fillMaxWidth()
      .height(48.dp)
      .then(if (borderColor != null) Modifier.border(1.dp, borderColor, shape) else Modifier)
      .background(backgroundColor, shape)
      .clickable(onClick = onClick),
  ) {
    AsyncImage(
      model = Res.getUri(svgPath),
      contentDescription = null,
      modifier = Modifier
        .align(Alignment.CenterStart)
        .padding(start = 24.dp)
        .size(20.dp),
      colorFilter = iconTint?.let { ColorFilter.tint(it) },
    )
    Text(
      text,
      style = AppTheme.typography.action,
      color = foregroundColor,
      modifier = Modifier.align(Alignment.Center),
    )
  }
}
