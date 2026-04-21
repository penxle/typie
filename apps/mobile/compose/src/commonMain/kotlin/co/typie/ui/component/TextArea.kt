package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.EaseInOutExpo
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.form.FieldState
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun TextArea(
  value: String,
  onValueChange: (String) -> Unit,
  modifier: Modifier = Modifier,
  label: String? = null,
  help: String? = null,
  error: String? = null,
  placeholder: String? = null,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  autoFocus: Boolean = false,
  onBlur: (() -> Unit)? = null,
  capitalization: KeyboardCapitalization = KeyboardCapitalization.Sentences,
  imeAction: ImeAction = ImeAction.Default,
  onImeAction: (() -> Unit)? = null,
  minLines: Int = 4,
  maxLines: Int = 6,
  minHeight: Dp = 112.dp,
) {
  val shape = AppShapes.rounded(AppShapes.md)
  var isFocused by remember { mutableStateOf(false) }
  val focusManager = LocalFocusManager.current
  val hasError = error != null
  val textInputState =
    rememberTextInputState(
      value = value,
      onValueChange = onValueChange,
      enabled = enabled && !readOnly,
      onDismiss = { focusManager.clearFocus() },
    )

  val colorSpec = tween<Color>(220)
  val containerColor by
    animateColorAsState(
      when {
        !enabled -> AppTheme.colors.surfaceCanvas
        isFocused -> AppTheme.colors.surfaceDefault
        else -> AppTheme.colors.surfaceInset
      },
      colorSpec,
    )
  val borderColor by
    animateColorAsState(
      when {
        hasError -> AppTheme.colors.danger
        isFocused -> AppTheme.colors.borderEmphasis
        else -> AppTheme.colors.borderHairline
      },
      colorSpec,
    )
  val borderWidth by
    animateDpAsState(
      if (isFocused || hasError) 1.5.dp else 1.dp,
      tween(durationMillis = 220, easing = EaseInOutExpo),
    )
  val helpColor by
    animateColorAsState(
      if (hasError) AppTheme.colors.danger else AppTheme.colors.textHint,
      colorSpec,
    )

  if (autoFocus) {
    LaunchedEffect(autoFocus) { textInputState.requestFocus() }
  }

  Column(modifier = modifier) {
    if (label != null) {
      Text(text = label, style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

      Spacer(Modifier.height(8.dp))
    }

    BasicTextField(
      value = textInputState.value,
      onValueChange = textInputState::onValueChange,
      enabled = enabled,
      readOnly = readOnly,
      modifier =
        Modifier.fillMaxWidth().heightIn(min = minHeight).textInputFocusable(
          textInputState,
          enabled = enabled && !readOnly,
        ) { state ->
          val wasFocused = isFocused
          isFocused = state.isFocused
          if (wasFocused && !state.isFocused) onBlur?.invoke()
        },
      textStyle =
        AppTheme.typography.body.copy(
          color = if (enabled) AppTheme.colors.textDefault else AppTheme.colors.textHint
        ),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      keyboardOptions = KeyboardOptions(capitalization = capitalization, imeAction = imeAction),
      keyboardActions =
        KeyboardActions(onNext = { onImeAction?.invoke() }, onDone = { onImeAction?.invoke() }),
      minLines = minLines,
      maxLines = maxLines,
      decorationBox = { innerTextField ->
        Box(
          modifier =
            Modifier.fillMaxWidth()
              .border(borderWidth, borderColor, shape)
              .background(containerColor, shape)
              .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
          if (textInputState.value.text.isEmpty() && placeholder != null) {
            Text(
              text = placeholder,
              style = AppTheme.typography.body,
              color = AppTheme.colors.textHint,
            )
          }

          innerTextField()
        }
      },
    )

    Spacer(Modifier.height(4.dp))

    Text(
      text = error ?: help ?: "",
      style = AppTheme.typography.micro,
      color = helpColor,
      modifier = Modifier.defaultMinSize(minHeight = 16.dp).padding(start = 8.dp),
    )
  }
}

@Composable
fun TextArea(
  field: FieldState<String>,
  modifier: Modifier = Modifier,
  label: String? = null,
  help: String? = null,
  placeholder: String? = null,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  autoFocus: Boolean = false,
  capitalization: KeyboardCapitalization = KeyboardCapitalization.Sentences,
  imeAction: ImeAction = ImeAction.Default,
  onImeAction: (() -> Unit)? = null,
  minLines: Int = 4,
  maxLines: Int = 6,
  minHeight: Dp = 112.dp,
) {
  val form = field.form
  val isSkeleton = LocalSkeleton.current.enabled
  val shouldAutoFocus = autoFocus || (form?.autoFocusFirstField == true && form.isFirstField(field))

  val resolvedOnImeAction: (() -> Unit)? =
    when {
      onImeAction != null -> onImeAction
      form != null && imeAction != ImeAction.Default -> {
        { form.focusNext(field) }
      }

      else -> null
    }

  if (shouldAutoFocus && !isSkeleton) {
    LaunchedEffect(shouldAutoFocus) { field.focusRequester.requestFocus() }
  }

  TextArea(
    value = field.value,
    onValueChange = { field.setValue(it) },
    modifier =
      if (!isSkeleton && (form != null || autoFocus)) {
        modifier.focusRequester(field.focusRequester)
      } else {
        modifier
      },
    label = label,
    help = help,
    error = field.errors.firstOrNull(),
    placeholder = placeholder,
    enabled = enabled,
    readOnly = readOnly,
    autoFocus = false,
    onBlur = { field.onBlur() },
    capitalization = capitalization,
    imeAction = imeAction,
    onImeAction = resolvedOnImeAction,
    minLines = minLines,
    maxLines = maxLines,
    minHeight = minHeight,
  )
}
