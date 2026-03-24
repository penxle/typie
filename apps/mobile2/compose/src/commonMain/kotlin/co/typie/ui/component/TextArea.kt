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
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.form.FieldState
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.theme.AppTheme
import androidx.compose.foundation.shape.RoundedCornerShape

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
  onBlur: (() -> Unit)? = null,
  capitalization: KeyboardCapitalization = KeyboardCapitalization.Sentences,
  imeAction: ImeAction = ImeAction.Default,
  onImeAction: (() -> Unit)? = null,
  minLines: Int = 4,
  maxLines: Int = 6,
  minHeight: Dp = 112.dp,
) {
  val shape = RoundedCornerShape(12.dp)
  var isFocused by remember { mutableStateOf(false) }
  val hasError = error != null

  val colorSpec = tween<Color>(220)
  val containerColor by animateColorAsState(
    when {
      !enabled -> AppTheme.colors.surfaceBase
      isFocused -> AppTheme.colors.surfaceDefault
      else -> AppTheme.colors.surfaceSunken
    },
    colorSpec,
  )
  val borderColor by animateColorAsState(
    when {
      hasError -> AppTheme.colors.dangerSubtle
      isFocused -> AppTheme.colors.borderStrong
      else -> AppTheme.colors.borderSubtle
    },
    colorSpec,
  )
  val borderWidth by animateDpAsState(
    if (isFocused || hasError) 1.5.dp else 1.dp,
    tween(durationMillis = 220, easing = EaseInOutExpo),
  )
  val helpColor by animateColorAsState(
    if (hasError) AppTheme.colors.dangerSubtle else AppTheme.colors.textMuted,
    colorSpec,
  )

  Column(modifier = modifier) {
    if (label != null) {
      Text(
        text = label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textSecondary,
      )

      Spacer(Modifier.height(8.dp))
    }

    BasicTextField(
      value = value,
      onValueChange = onValueChange,
      enabled = enabled,
      readOnly = readOnly,
      modifier = Modifier
        .fillMaxWidth()
        .heightIn(min = minHeight)
        .onFocusChanged { state ->
          val wasFocused = isFocused
          isFocused = state.isFocused
          if (wasFocused && !state.isFocused) onBlur?.invoke()
        },
      textStyle = AppTheme.typography.body.copy(
        color = if (enabled) AppTheme.colors.textPrimary else AppTheme.colors.textMuted,
      ),
      cursorBrush = SolidColor(AppTheme.colors.textPrimary),
      keyboardOptions = KeyboardOptions(
        capitalization = capitalization,
        imeAction = imeAction,
      ),
      keyboardActions = KeyboardActions(
        onNext = { onImeAction?.invoke() },
        onDone = { onImeAction?.invoke() },
      ),
      minLines = minLines,
      maxLines = maxLines,
      decorationBox = { innerTextField ->
        Box(
          modifier = Modifier
            .fillMaxWidth()
            .border(borderWidth, borderColor, shape)
            .background(containerColor, shape)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        ) {
          if (value.isEmpty() && placeholder != null) {
            Text(
              text = placeholder,
              style = AppTheme.typography.body,
              color = AppTheme.colors.textMuted,
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
      modifier = Modifier
        .defaultMinSize(minHeight = 16.dp)
        .padding(start = 8.dp),
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
  capitalization: KeyboardCapitalization = KeyboardCapitalization.Sentences,
  imeAction: ImeAction = ImeAction.Default,
  onImeAction: (() -> Unit)? = null,
  minLines: Int = 4,
  maxLines: Int = 6,
  minHeight: Dp = 112.dp,
) {
  val form = field.form
  val isSkeleton = LocalSkeleton.current.enabled

  val resolvedOnImeAction: (() -> Unit)? = when {
    onImeAction != null -> onImeAction
    form != null && imeAction != ImeAction.Default -> {
      { form.focusNext(field) }
    }

    else -> null
  }

  if (form != null && form.isFirstField(field) && !isSkeleton) {
    LaunchedEffect(Unit) {
      field.focusRequester.requestFocus()
    }
  }

  TextArea(
    value = field.value,
    onValueChange = { field.setValue(it) },
    modifier = if (form != null && !isSkeleton) {
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
    onBlur = { field.onBlur() },
    capitalization = capitalization,
    imeAction = imeAction,
    onImeAction = resolvedOnImeAction,
    minLines = minLines,
    maxLines = maxLines,
    minHeight = minHeight,
  )
}
