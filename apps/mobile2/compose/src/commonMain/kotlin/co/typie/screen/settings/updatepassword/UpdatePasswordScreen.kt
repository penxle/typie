package co.typie.screen.settings.updatepassword

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun UpdatePasswordScreen() {
  val nav = Nav.current
  val model = viewModel { UpdatePasswordViewModel() }
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val hasPassword = model.query.data.me.hasPassword
  val buttonText = if (hasPassword) "변경" else "설정"
  val loadingText = if (hasPassword) "변경 중..." else "설정 중..."

  fun submit() {
    scope.launch {
      model.submit().withDefaultExceptionHandler(toast).onOk {
        toast.show(ToastType.Success, "비밀번호가 변경되었어요.")
        nav.pop()
      }
    }
  }

  val dialog = LocalDialog.current

  ProvideTopBar(center = { Text("비밀번호 변경", style = AppTheme.typography.title) })

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)) {
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
        onImeAction = { submit() },
      )

      Spacer(Modifier.height(24.dp))

      Button(
        text = buttonText,
        modifier = Modifier.padding(horizontal = 16.dp).padding(bottom = 16.dp),
        loading = model.isSubmitting,
        loadingText = loadingText,
        onClick = { submit() },
      )
    }
  }
}
