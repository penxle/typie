package co.typie.screen.login_with_email

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.ui.clickable
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.SuitFontFamily
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginWithEmailScreen() {
  val nav = Nav.current
  val viewModel = koinViewModel<LoginWithEmailViewModel>()
  val state by viewModel.state.collectAsState()
  val passwordFocusRequester = remember { FocusRequester() }

  Screen {
    Column(
      modifier = Modifier
        .fillMaxSize()
        .windowInsetsPadding(WindowInsets.safeDrawing),
    ) {
      // Top bar
      Box(
        modifier = Modifier
          .fillMaxWidth()
          .height(56.dp)
          .padding(horizontal = 20.dp),
      ) {
        Box(
          modifier = Modifier
            .align(Alignment.CenterStart)
            .size(24.dp)
            .clickable { nav.pop() },
        ) {
          Icon(Lucide.ArrowLeft)
        }
        Text(
          "이메일로 로그인",
          style = TextStyle(fontSize = 17.sp, fontWeight = FontWeight.W600),
          modifier = Modifier.align(Alignment.Center),
        )
      }

      // Form fields
      Column(
        modifier = Modifier.padding(horizontal = 20.dp),
      ) {
        Spacer(Modifier.height(24.dp))

        FormField(
          label = "이메일",
          value = state.email,
          onValueChange = viewModel::setEmail,
          placeholder = "me@example.com",
          keyboardType = KeyboardType.Email,
          imeAction = ImeAction.Next,
          onNext = { passwordFocusRequester.requestFocus() },
          onEnter = { passwordFocusRequester.requestFocus() },
          error = state.emailError,
        )

        Spacer(Modifier.height(16.dp))

        FormField(
          label = "비밀번호",
          value = state.password,
          onValueChange = viewModel::setPassword,
          placeholder = "********",
          keyboardType = KeyboardType.Password,
          isPassword = true,
          imeAction = ImeAction.Done,
          onDone = viewModel::submit,
          onEnter = viewModel::submit,
          error = state.passwordError,
          modifier = Modifier.focusRequester(passwordFocusRequester),
        )

      }

      Spacer(Modifier.weight(1f))

      // Bottom button
      Box(
        modifier = Modifier
          .fillMaxWidth()
          .padding(horizontal = 20.dp)
          .padding(bottom = 16.dp)
          .height(48.dp)
          .background(AppTheme.colors.accentBrand, RoundedCornerShape(999.dp))
          .clickable { viewModel.submit() },
        contentAlignment = Alignment.Center,
      ) {
        Text(
          "로그인",
          style = TextStyle(
            fontSize = 15.sp,
            fontWeight = FontWeight.W600,
            color = AppTheme.colors.textBright,
          ),
        )
      }
    }
  }
}

@Composable
private fun FormField(
  label: String,
  value: String,
  onValueChange: (String) -> Unit,
  placeholder: String,
  keyboardType: KeyboardType = KeyboardType.Text,
  isPassword: Boolean = false,
  imeAction: ImeAction = ImeAction.Default,
  onNext: (() -> Unit)? = null,
  onDone: (() -> Unit)? = null,
  onEnter: (() -> Unit)? = null,
  error: String? = null,
  modifier: Modifier = Modifier,
) {
  val shape = RoundedCornerShape(12.dp)
  val borderColor =
    if (error != null) AppTheme.colors.borderDanger else AppTheme.colors.borderDefault

  Column {
    Text(
      label,
      style = TextStyle(fontSize = 14.sp, fontWeight = FontWeight.W500),
    )
    Spacer(Modifier.height(8.dp))
    BasicTextField(
      value = value,
      onValueChange = onValueChange,
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
      textStyle = TextStyle(
        fontFamily = SuitFontFamily,
        fontSize = 15.sp,
        color = AppTheme.colors.textDefault,
      ),
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
          modifier = Modifier
            .fillMaxWidth()
            .height(48.dp)
            .border(1.dp, borderColor, shape)
            .background(AppTheme.colors.surfaceDefault, shape)
            .padding(horizontal = 16.dp),
          contentAlignment = Alignment.CenterStart,
        ) {
          if (value.isEmpty()) {
            Text(
              placeholder,
              style = TextStyle(fontSize = 15.sp, color = AppTheme.colors.textDisabled),
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
        style = TextStyle(fontSize = 12.sp, color = AppTheme.colors.textDanger),
      )
    }
  }
}
