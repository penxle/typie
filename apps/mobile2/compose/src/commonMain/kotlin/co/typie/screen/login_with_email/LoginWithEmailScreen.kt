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
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
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
          error = state.emailError,
          enabled = !state.isLoading,
        )

        Spacer(Modifier.height(16.dp))

        FormField(
          label = "비밀번호",
          value = state.password,
          onValueChange = viewModel::setPassword,
          placeholder = "********",
          keyboardType = KeyboardType.Password,
          isPassword = true,
          error = state.passwordError,
          enabled = !state.isLoading,
        )

      }

      Spacer(Modifier.weight(1f))

      // Bottom button
      val buttonColor = if (state.isLoading) AppTheme.colors.interactiveDisabled else AppTheme.colors.accentBrand
      val buttonTextColor = if (state.isLoading) AppTheme.colors.textDisabled else AppTheme.colors.textBright

      Box(
        modifier = Modifier
          .fillMaxWidth()
          .padding(horizontal = 20.dp)
          .padding(bottom = 16.dp)
          .height(48.dp)
          .background(buttonColor, RoundedCornerShape(999.dp))
          .then(if (!state.isLoading) Modifier.clickable { viewModel.submit() } else Modifier),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          if (state.isLoading) "로그인 중..." else "로그인",
          style = TextStyle(
            fontSize = 15.sp,
            fontWeight = FontWeight.W600,
            color = buttonTextColor,
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
  error: String? = null,
  enabled: Boolean = true,
) {
  val shape = RoundedCornerShape(12.dp)
  val borderColor = if (error != null) AppTheme.colors.borderDanger else AppTheme.colors.borderDefault

  Column {
    Text(
      label,
      style = TextStyle(fontSize = 14.sp, fontWeight = FontWeight.W500),
    )
    Spacer(Modifier.height(8.dp))
    BasicTextField(
      value = value,
      onValueChange = onValueChange,
      enabled = enabled,
      textStyle = TextStyle(
        fontFamily = SuitFontFamily,
        fontSize = 15.sp,
        color = AppTheme.colors.textDefault,
      ),
      keyboardOptions = KeyboardOptions(keyboardType = keyboardType),
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
