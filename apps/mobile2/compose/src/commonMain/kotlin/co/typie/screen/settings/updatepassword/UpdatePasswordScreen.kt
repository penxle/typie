package co.typie.screen.settings.updatepassword

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.result.loading
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun UpdatePasswordScreen() {
  val model = viewModel { UpdatePasswordViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  fun submit() {
    scope.launch {
      model.submit().withDefaultExceptionHandler(toast).onOk {
        toast.success("비밀번호가 변경되었어요.")
        nav.pop()
      }
    }
  }

  ProvideTopBar(center = { Text("비밀번호 변경", style = AppTheme.typography.title) })

  Screen(loadable = model.query) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding)) {
      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        if (model.query.data.me.hasPassword) {
          TextField(
            field = model.form.currentPassword,
            label = "현재 비밀번호",
            labelPosition = LabelPosition.Internal,
            placeholder = "********",
            isPassword = true,
            contentType = ContentType.Password,
          )
        }

        TextField(
          field = model.form.newPassword,
          label = "새 비밀번호",
          labelPosition = LabelPosition.Internal,
          placeholder = "********",
          isPassword = true,
          contentType = ContentType.NewPassword,
        )

        TextField(
          field = model.form.confirmPassword,
          label = "새 비밀번호 확인",
          labelPosition = LabelPosition.Internal,
          placeholder = "********",
          isPassword = true,
          contentType = ContentType.NewPassword,
          onImeAction = { submit() },
        )
      }

      ToastAnchor()

      Button(
        text = if (model.query.data.me.hasPassword) "변경" else "설정",
        loading = model.isSubmitting,
        loadingText = if (model.query.data.me.hasPassword) "변경 중..." else "설정 중...",
        onClick = { submit() },
      )
    }
  }
}
