package co.typie.screen.update_password

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.ui.component.Button
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun UpdatePasswordScreen() {
  val nav = Nav.current
  val model = koinViewModel<UpdatePasswordViewModel>()
  val hasPassword = model.query.data.me.hasPassword
  val buttonText = if (hasPassword) "변경" else "설정"
  val loadingText = if (hasPassword) "변경 중..." else "설정 중..."

  ProvideTopBar(
    center = { Text("비밀번호 변경", style = AppTheme.typography.title) },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
  ) { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .navigationBarsPadding()
        .imePadding(),
    ) {
      if (hasPassword) {
        TextField(
          field = model.state.form.currentPassword,
          label = "현재 비밀번호",
          labelPosition = LabelPosition.Internal,
          placeholder = "현재 비밀번호를 입력하세요",
          isPassword = true,
          contentType = ContentType.Password,
        )
      }

      TextField(
        field = model.state.form.newPassword,
        label = "새 비밀번호",
        labelPosition = LabelPosition.Internal,
        placeholder = "********",
        isPassword = true,
        contentType = ContentType.NewPassword,
      )

      TextField(
        field = model.state.form.confirmPassword,
        label = "새 비밀번호 확인",
        labelPosition = LabelPosition.Internal,
        placeholder = "********",
        isPassword = true,
        contentType = ContentType.NewPassword,
        onImeAction = { model.submit { nav.pop() } },
      )

      Spacer(Modifier.weight(1f))

      Button(
        text = buttonText,
        modifier = Modifier.padding(bottom = 16.dp),
        loading = model.state.isSubmitting,
        loadingText = loadingText,
        onClick = { model.submit { nav.pop() } },
      )
    }
  }
}
