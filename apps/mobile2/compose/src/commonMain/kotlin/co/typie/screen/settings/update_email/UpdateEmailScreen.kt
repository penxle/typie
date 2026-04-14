package co.typie.screen.settings.update_email

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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.result.DEFAULT_ERROR_MESSAGE
import co.typie.result.onErr
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.dialog.error
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun UpdateEmailScreen() {
  val nav = Nav.current
  val model = viewModel { UpdateEmailViewModel() }
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val showSuccessModal: suspend () -> Unit = {
    dialog.alert(title = "이메일 인증", message = "변경할 이메일 주소로 인증 메일을 발송했어요. 메일함을 확인해주세요.")
    nav.pop()
  }

  fun submit() {
    scope.launch {
      model
        .submit()
        .withDefaultExceptionHandler(toast)
        .onOk { showSuccessModal() }
        .onErr { error ->
          val message =
            when (error) {
              UpdateEmailError.EmailAlreadyExists -> "이미 사용중인 이메일이에요."
              is UpdateEmailError.Unknown -> DEFAULT_ERROR_MESSAGE
            }
          toast.show(ToastType.Error, message)
        }
    }
  }

  ProvideTopBar(center = { Text("이메일 변경", style = AppTheme.typography.title) })

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    imeAware = true,
    bottomBar = {
      Button(
        text = "변경",
        modifier = Modifier.padding(horizontal = 16.dp).padding(bottom = 16.dp),
        loading = model.isSubmitting,
        loadingText = "변경 중...",
        onClick = { submit() },
      )
    },
  ) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)) {
      Text("현재 이메일 주소", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)

      Text(model.query.data.me.email, style = AppTheme.typography.action)

      Spacer(Modifier.height(8.dp))

      TextField(
        field = model.state.form.email,
        label = "변경할 이메일 주소",
        labelPosition = LabelPosition.Internal,
        placeholder = "me@example.com",
        contentType = ContentType.EmailAddress,
        keyboardType = KeyboardType.Email,
        onImeAction = { submit() },
      )
    }
  }
}
