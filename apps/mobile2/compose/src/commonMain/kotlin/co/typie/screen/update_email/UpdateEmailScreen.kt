package co.typie.screen.update_email

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.ui.component.AlertModal
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
fun UpdateEmailScreen() {
  val nav = Nav.current
  val model = koinViewModel<UpdateEmailViewModel>()
  val showSuccessModal: suspend () -> Unit = {
    nav.showModal {
      AlertModal(
        title = "이메일 인증",
        message = "변경할 이메일 주소로 인증 메일을 발송했어요. 메일함을 확인해주세요.",
        onConfirm = {
          nav.dismissModal()
          nav.pop()
        },
        onDismiss = {
          nav.dismissModal()
          nav.pop()
        },
      )
    }
  }

  ProvideTopBar(
    center = { Text("이메일 변경", style = AppTheme.typography.title) },
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
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(
        "현재 이메일 주소",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      Text(
        model.query.data.me.email,
        style = AppTheme.typography.action,
      )

      Spacer(Modifier.height(8.dp))

      TextField(
        field = model.state.form.email,
        label = "변경할 이메일 주소",
        labelPosition = LabelPosition.Internal,
        placeholder = "me@example.com",
        contentType = ContentType.EmailAddress,
        keyboardType = KeyboardType.Email,
        onImeAction = { model.submit(showSuccessModal) },
      )

      Spacer(Modifier.weight(1f))

      Button(
        text = "변경",
        modifier = Modifier.padding(bottom = 16.dp),
        loading = model.state.isSubmitting,
        loadingText = "변경 중...",
        onClick = { model.submit(showSuccessModal) },
      )
    }
  }
}
