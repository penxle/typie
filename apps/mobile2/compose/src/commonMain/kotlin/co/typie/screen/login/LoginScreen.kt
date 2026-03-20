package co.typie.screen.login

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.auth.sso.activityContext
import co.typie.di.Platform
import co.typie.generated.resources.Res
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.clickable
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginScreen() {
  val nav = Nav.current
  val viewModel = koinViewModel<LoginViewModel>()
  val state by viewModel.state.collectAsState()
  val platform = koinInject<Platform>()
  val ctx = activityContext()

  Screen {
    Column(
      modifier = Modifier
        .fillMaxSize()
        .windowInsetsPadding(WindowInsets.safeDrawing),
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
        Text("작성, 정리, 공유까지.", style = TextStyle(fontSize = 16.sp))
        Spacer(Modifier.height(4.dp))
        Text("글쓰기의 모든 과정을", style = TextStyle(fontSize = 16.sp, fontWeight = FontWeight.W700))
        Spacer(Modifier.height(4.dp))
        Text("타이피 하나로 해결해요.", style = TextStyle(fontSize = 16.sp, fontWeight = FontWeight.W700))
      }

      Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        if (platform != Platform.Jvm) {
          SsoButton(
            text = "구글로 시작하기",
            svgPath = "files/brands/google.svg",
            foregroundColor = Color(0xFF000000),
            backgroundColor = Color(0xFFFFFFFF),
            borderColor = AppTheme.colors.borderDefault,
            enabled = !state.isLoading,
            onClick = { viewModel.loginWithGoogle(ctx) },
          )
          SsoButton(
            text = "카카오로 시작하기",
            svgPath = "files/brands/kakao.svg",
            iconTint = Color(0xFF000000),
            foregroundColor = Color(0xFF000000),
            backgroundColor = Color(0xFFFEE500),
            enabled = !state.isLoading,
            onClick = { viewModel.loginWithKakao(ctx) },
          )
          SsoButton(
            text = "네이버로 시작하기",
            svgPath = "files/brands/naver.svg",
            iconTint = Color(0xFFFFFFFF),
            foregroundColor = Color(0xFFFFFFFF),
            backgroundColor = Color(0xFF03C75A),
            enabled = !state.isLoading,
            onClick = { viewModel.loginWithNaver(ctx) },
          )

          if (platform == Platform.iOS) {
            SsoButton(
              text = "애플로 시작하기",
              svgPath = "files/brands/apple.svg",
              iconTint = Color(0xFFFFFFFF),
              foregroundColor = Color(0xFFFFFFFF),
              backgroundColor = Color(0xFF000000),
              enabled = !state.isLoading,
              onClick = { viewModel.loginWithApple(ctx) },
            )
          }
        }

        Text(
          "이메일로 가입하셨나요?",
          style = TextStyle(fontSize = 14.sp, color = AppTheme.colors.textSubtle),
          modifier = Modifier
            .padding(horizontal = 24.dp, vertical = 8.dp)
            .clickable { nav.navigate(Route.LoginWithEmail) },
        )
      }
    }
  }
}

@Composable
private fun SsoButton(
  text: String,
  svgPath: String,
  foregroundColor: Color,
  backgroundColor: Color,
  borderColor: Color? = null,
  iconTint: Color? = null,
  enabled: Boolean = true,
  onClick: () -> Unit = {},
) {
  val shape = RoundedCornerShape(999.dp)
  val alpha = if (enabled) 1f else 0.5f

  Box(
    modifier = Modifier
      .fillMaxWidth()
      .height(48.dp)
      .alpha(alpha)
      .then(if (borderColor != null) Modifier.border(1.dp, borderColor, shape) else Modifier)
      .background(backgroundColor, shape)
      .then(if (enabled) Modifier.clickable(onClick = onClick) else Modifier),
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
      style = TextStyle(fontSize = 15.sp, lineHeight = 15.sp, fontWeight = FontWeight.W600, color = foregroundColor),
      modifier = Modifier.align(Alignment.Center),
    )
  }
}
