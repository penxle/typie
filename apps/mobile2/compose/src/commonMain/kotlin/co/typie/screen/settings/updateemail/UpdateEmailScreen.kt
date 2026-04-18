package co.typie.screen.settings.updateemail

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.excludeTop
import co.typie.ext.imePadding
import co.typie.ext.onlyTop
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun UpdateEmailScreen() {
  val model = viewModel { UpdateEmailViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current
  val dialog = LocalDialog.current

  fun submit() {
    scope.launch {
      model.submit().withDefaultExceptionHandler(toast).onOk {
        dialog.alert(title = "이메일 인증", message = "변경할 이메일 주소로 인증 메일을 발송했어요. 메일함을 확인해주세요.")
        nav.pop()
      }
    }
  }

  ProvideTopBar(center = { Text("이메일 변경", style = AppTheme.typography.title) })

  Screen(loadable = model.query) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().imePadding().padding(contentPadding.excludeTop())) {
      Box(modifier = Modifier.weight(1f)) {
        Column(
          modifier =
            Modifier.fillMaxSize()
              .verticalScroll(scrollState)
              .padding(contentPadding.onlyTop())
              .padding(AppTheme.spacings.scrollBottomPadding)
        ) {
          Text(
            "현재 이메일 주소",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textSecondary,
          )

          Spacer(Modifier.height(8.dp))

          Text(model.query.data.me.email, style = AppTheme.typography.action)

          Spacer(Modifier.height(20.dp))

          TextField(
            field = model.form.email,
            label = "변경할 이메일 주소",
            labelPosition = LabelPosition.External,
            placeholder = "me@example.com",
            contentType = ContentType.EmailAddress,
            keyboardType = KeyboardType.Email,
            onImeAction = { submit() },
          )
        }
      }

      ToastAnchor()

      Button(
        text = "변경",
        loading = model.isSubmitting,
        loadingText = "변경 중...",
        onClick = { submit() },
      )
    }
  }
}
