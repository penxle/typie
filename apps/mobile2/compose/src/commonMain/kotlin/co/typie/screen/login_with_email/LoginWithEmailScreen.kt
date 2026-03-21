package co.typie.screen.login_with_email

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsPadding
import co.typie.form.FieldState
import co.typie.navigation.Nav
import co.typie.ui.component.Button
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginWithEmailScreen() {
  val viewModel = koinViewModel<LoginWithEmailViewModel>()
  val form = viewModel.state.form
  val passwordFocusRequester = remember { FocusRequester() }

  ProvideTopBar(
    center = { Text("이메일로 로그인", style = AppTheme.typography.title) },
  )

  Screen { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding).navigationBarsPadding()) {
      EmailFormField(
        label = "이메일",
        field = form.email,
        placeholder = "me@example.com",
        keyboardType = KeyboardType.Email,
        imeAction = ImeAction.Next,
        onNext = { passwordFocusRequester.requestFocus() },
        onEnter = { passwordFocusRequester.requestFocus() },
      )

      Spacer(Modifier.height(16.dp))

      EmailFormField(
        label = "비밀번호",
        field = form.password,
        placeholder = "********",
        keyboardType = KeyboardType.Password,
        isPassword = true,
        imeAction = ImeAction.Done,
        onDone = viewModel::submit,
        onEnter = viewModel::submit,
        modifier = Modifier.focusRequester(passwordFocusRequester),
      )

      Spacer(Modifier.weight(1f))

      Button(
        text = "로그인",
        onClick = viewModel::submit,
        modifier = Modifier.padding(bottom = 16.dp),
        loading = form.isProcessing,
        loadingText = "로그인 중...",
      )
    }
  }
}

@Composable
private fun EmailFormField(
  label: String,
  field: FieldState<String>,
  placeholder: String,
  keyboardType: KeyboardType = KeyboardType.Text,
  isPassword: Boolean = false,
  imeAction: ImeAction = ImeAction.Default,
  onNext: (() -> Unit)? = null,
  onDone: (() -> Unit)? = null,
  onEnter: (() -> Unit)? = null,
  modifier: Modifier = Modifier,
) {
  val shape = RoundedCornerShape(12.dp)
  val error = field.errors.firstOrNull()
  val borderColor =
    if (error != null) AppTheme.colors.borderDanger else AppTheme.colors.borderDefault

  Column {
    Text(
      label,
      style = AppTheme.typography.caption,
    )
    Spacer(Modifier.height(8.dp))
    BasicTextField(
      value = field.value,
      onValueChange = { field.setValue(it) },
      modifier = modifier.onPreviewKeyEvent {
        if (it.type != KeyEventType.KeyDown) {
          false
        } else {
          when (it.key) {
            Key.Enter -> {
              onEnter?.invoke()
              onEnter != null
            }

            else -> false
          }
        }
      },
      textStyle = AppTheme.typography.action.copy(color = AppTheme.colors.textDefault),
      keyboardOptions = KeyboardOptions(
        keyboardType = keyboardType,
        imeAction = imeAction,
      ),
      keyboardActions = KeyboardActions(
        onNext = { onNext?.invoke() },
        onDone = { onDone?.invoke() },
      ),
      visualTransformation = if (isPassword) PasswordVisualTransformation() else VisualTransformation.None,
      singleLine = true,
      decorationBox = { innerTextField ->
        Box(
          modifier = Modifier.fillMaxWidth().height(48.dp).border(1.dp, borderColor, shape)
            .background(AppTheme.colors.surfaceDefault, shape).padding(horizontal = 16.dp),
          contentAlignment = Alignment.CenterStart,
        ) {
          if (field.value.isEmpty()) {
            Text(
              placeholder,
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textDisabled,
            )
          }
          innerTextField()
        }
      },
    )
    if (error != null) {
      Spacer(Modifier.height(4.dp))
      Text(
        error,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textDanger,
      )
    }
  }
}
