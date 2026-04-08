package co.typie.screen.login

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import co.typie.auth.sso.activityContext
import co.typie.di.Platform
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.safeBottomPadding
import co.typie.generated.resources.Res
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.ui.component.Button
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginScreen() {
  val bottomSheetHost = LocalBottomSheetHost.current

  ProvideTopBar(
    enabled = false
  )

  Screen(body = { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .safeBottomPadding(),
    ) {
      Column(
        modifier = Modifier.weight(1f).fillMaxWidth(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        Img(
          url = Res.getUri("files/logos/full.svg"),
          modifier = Modifier.height(32.dp),
          contentScale = ContentScale.FillHeight,
          color = AppTheme.colors.textPrimary,
        )
        Spacer(Modifier.height(24.dp))
        Text("작성, 정리, 공유까지.", style = AppTheme.typography.label)
        Spacer(Modifier.height(4.dp))
        Text("글쓰기의 모든 과정을", style = AppTheme.typography.label)
        Spacer(Modifier.height(4.dp))
        Text("타이피 하나로 해결해요.", style = AppTheme.typography.label)
      }

      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        Button(
          text = "시작하기",
          onClick = { bottomSheetHost.show { LoginBottomSheet() } },
        )
      }
    }
  })
}

private enum class LoginStep { SingleSignOn, Email }

@Composable
fun BottomSheetScope<Unit>.LoginBottomSheet() {
  var step by remember { mutableStateOf(LoginStep.SingleSignOn) }

  BottomSheetScaffold(title = "로그인") {
    AnimatedContent(
      targetState = step,
      transitionSpec = {
        if (targetState == LoginStep.Email) {
          slideInHorizontally { it } togetherWith slideOutHorizontally { -it }
        } else {
          slideInHorizontally { -it } togetherWith slideOutHorizontally { it }
        }
      },
    ) { currentStep ->
      when (currentStep) {
        LoginStep.SingleSignOn -> LoginSSOContent(
          onEmailClick = { step = LoginStep.Email },
          onSuccess = { dismiss() },
        )

        LoginStep.Email -> LoginEmailContent(
          onBack = { step = LoginStep.SingleSignOn },
          onSuccess = { dismiss() },
        )
      }
    }
  }
}

@Composable
private fun LoginSSOContent(
  onEmailClick: () -> Unit,
  onSuccess: () -> Unit,
) {
  val model = koinViewModel<LoginSingleSignOnViewModel>()
  val platform = koinInject<Platform>()
  val ctx = activityContext()

  Column(
    modifier = Modifier.fillMaxWidth(),
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    Column(
      modifier = Modifier.fillMaxWidth(),
      verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
      SingleSignOnButton(
        text = "구글로 시작하기",
        svgPath = "files/brands/google.svg",
        foregroundColor = Color(0xFF000000),
        backgroundColor = Color(0xFFFFFFFF),
        borderColor = AppTheme.colors.borderDefault,
        onClick = { model.loginWith(SingleSignOnProvider.GOOGLE, ctx, onSuccess) },
      )

      SingleSignOnButton(
        text = "카카오로 시작하기",
        svgPath = "files/brands/kakao.svg",
        iconTint = Color(0xFF000000),
        foregroundColor = Color(0xFF000000),
        backgroundColor = Color(0xFFFEE500),
        onClick = { model.loginWith(SingleSignOnProvider.KAKAO, ctx, onSuccess) },
      )

      SingleSignOnButton(
        text = "네이버로 시작하기",
        svgPath = "files/brands/naver.svg",
        iconTint = Color(0xFFFFFFFF),
        foregroundColor = Color(0xFFFFFFFF),
        backgroundColor = Color(0xFF03C75A),
        onClick = { model.loginWith(SingleSignOnProvider.NAVER, ctx, onSuccess) },
      )

      if (platform != Platform.Android) {
        SingleSignOnButton(
          text = "애플로 시작하기",
          svgPath = "files/brands/apple.svg",
          iconTint = Color(0xFFFFFFFF),
          foregroundColor = Color(0xFFFFFFFF),
          backgroundColor = Color(0xFF000000),
          onClick = { model.loginWith(SingleSignOnProvider.APPLE, ctx, onSuccess) },
        )
      }
    }

    Text(
      "이메일로 가입하셨나요?",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textSecondary,
      modifier = Modifier
        .padding(vertical = 16.dp)
        .clickable { onEmailClick() },
    )
  }
}

@Composable
private fun LoginEmailContent(
  onBack: () -> Unit,
  onSuccess: () -> Unit,
) {
  val model = koinViewModel<LoginWithEmailViewModel>()
  val form = model.state.form

  Column(
    modifier = Modifier.fillMaxWidth(),
  ) {
    TextField(
      field = form.email,
      label = "이메일",
      placeholder = "me@example.com",
      contentType = ContentType.Username + ContentType.EmailAddress,
      keyboardType = KeyboardType.Email,
    )

    Spacer(Modifier.height(8.dp))

    TextField(
      field = form.password,
      label = "비밀번호",
      placeholder = "********",
      isPassword = true,
      contentType = ContentType.Password,
      onImeAction = { model.submit(onSuccess) },
    )

    Spacer(Modifier.height(8.dp))

    Button(
      text = "로그인",
      onClick = { model.submit(onSuccess) },
      loading = model.submitAction.running,
      loadingText = "로그인 중...",
    )

    Text(
      "다른 방법으로 로그인",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textSecondary,
      modifier = Modifier
        .align(Alignment.CenterHorizontally)
        .padding(vertical = 16.dp)
        .clickable { onBack() },
    )
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

  InteractionScope {
    Box(
      modifier = Modifier
        .fillMaxWidth()
        .height(48.dp)
        .then(if (borderColor != null) Modifier.border(1.dp, borderColor, shape) else Modifier)
        .background(backgroundColor, shape)
        .clickable(onClick = onClick),
    ) {
      Img(
        url = Res.getUri(svgPath),
        modifier = Modifier
          .align(Alignment.CenterStart)
          .padding(start = 24.dp)
          .size(20.dp),
        color = iconTint,
      )

      Text(
        text,
        style = AppTheme.typography.action,
        color = foregroundColor,
        modifier = Modifier.align(Alignment.Center).pressScale(),
      )
    }
  }
}
