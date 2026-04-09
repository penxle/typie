package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.EaseInOutExpo
import androidx.compose.animation.core.EaseOutExpo
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.autofill.contentType
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import co.typie.form.FieldState
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.theme.AppTheme

enum class LabelPosition {
  External,
  Internal,
  None,
}

internal fun resolveTextFieldKeyboardType(
  isPassword: Boolean,
  keyboardType: KeyboardType,
): KeyboardType {
  return if (isPassword && keyboardType == KeyboardType.Text) KeyboardType.Password else keyboardType
}

internal fun resolveTextFieldAutofillContentType(
  isPassword: Boolean,
  contentType: ContentType?,
): ContentType? {
  if (contentType != null) return contentType
  return if (isPassword) ContentType.Password else null
}

@Composable
fun TextField(
  value: String,
  onValueChange: (String) -> Unit,
  label: String,
  modifier: Modifier = Modifier,
  help: String? = null,
  helpTextStyle: TextStyle = AppTheme.typography.micro,
  error: String? = null,
  success: Boolean = false,
  placeholder: String? = null,
  labelPosition: LabelPosition = LabelPosition.External,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  autoFocus: Boolean = false,
  isPassword: Boolean = false,
  contentType: ContentType? = null,
  onBlur: (() -> Unit)? = null,
  keyboardType: KeyboardType = KeyboardType.Text,
  imeAction: ImeAction? = null,
  onImeAction: (() -> Unit)? = null,
  leadingIcon: @Composable (() -> Unit)? = null,
  suffix: @Composable (() -> Unit)? = null,
) {
  val shape = RoundedCornerShape(12.dp)
  var isFocused by remember { mutableStateOf(false) }
  val focusRequester = remember { FocusRequester() }

  var textFieldValue by remember {
    mutableStateOf(TextFieldValue(value, TextRange(value.length)))
  }

  if (textFieldValue.text != value) {
    textFieldValue = TextFieldValue(value, TextRange(value.length))
  }

  val hasError = error != null
  val isInternal = labelPosition == LabelPosition.Internal
  val resolvedImeAction = imeAction ?: ImeAction.Default
  val resolvedKeyboardType = resolveTextFieldKeyboardType(isPassword, keyboardType)
  val resolvedContentType = resolveTextFieldAutofillContentType(isPassword, contentType)

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
      hasError -> AppTheme.colors.danger
      isFocused -> AppTheme.colors.borderStrong
      else -> AppTheme.colors.borderSubtle
    },
    colorSpec,
  )

  val borderWidth by animateDpAsState(
    if (isFocused || hasError) 1.5.dp else 1.dp,
    tween(durationMillis = 220, easing = EaseInOutExpo),
  )

  val horizontalPadding = 16.dp
  val verticalPadding = 8.dp
  val labelTopPadding = 10.dp

  val labelColor by animateColorAsState(
    when {
      hasError -> AppTheme.colors.danger
      isInternal -> AppTheme.colors.textTertiary
      isFocused -> AppTheme.colors.textPrimary
      else -> AppTheme.colors.textSecondary
    },
    colorSpec,
  )

  val labelActive = isInternal && (isFocused || value.isNotEmpty())
  val fieldHeight = if (isInternal) 56.dp else 48.dp

  if (autoFocus) {
    LaunchedEffect(autoFocus) {
      focusRequester.requestFocus()
    }
  }

  val labelProgress by animateFloatAsState(
    if (labelActive) 1f else 0f,
    tween(durationMillis = 220, easing = EaseOutExpo)
  )

  Column(modifier = modifier) {
    if (!isInternal && labelPosition != LabelPosition.None) {
      Text(
        label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textSecondary,
      )

      Spacer(Modifier.height(8.dp))
    }

    BasicTextField(
      value = textFieldValue,
      onValueChange = { newValue ->
        textFieldValue = newValue
        onValueChange(newValue.text)
      },
      enabled = enabled,
      readOnly = readOnly,
      modifier = Modifier
        .then(
          if (resolvedContentType != null) {
            Modifier.contentType(resolvedContentType)
          } else {
            Modifier
          }
        )
        .then(if (autoFocus) Modifier.focusRequester(focusRequester) else Modifier)
        .onFocusChanged { state ->
          val wasFocused = isFocused
          isFocused = state.isFocused
          if (wasFocused && !state.isFocused) onBlur?.invoke()
        }
        .onPreviewKeyEvent {
          if (it.type == KeyEventType.KeyDown && it.key == Key.Enter) {
            onImeAction?.invoke()
            onImeAction != null
          } else {
            false
          }
        },
      textStyle = AppTheme.typography.body.copy(
        color = if (enabled) AppTheme.colors.textPrimary else AppTheme.colors.textMuted,
      ),
      cursorBrush = SolidColor(AppTheme.colors.textPrimary),
      keyboardOptions = KeyboardOptions(
        keyboardType = resolvedKeyboardType,
        imeAction = resolvedImeAction,
      ),
      keyboardActions = KeyboardActions(
        onNext = { onImeAction?.invoke() },
        onDone = { onImeAction?.invoke() },
      ),
      visualTransformation = if (isPassword) PasswordVisualTransformation() else VisualTransformation.None,
      singleLine = true,
      decorationBox = { innerTextField ->
        Box(
          modifier = Modifier
            .fillMaxWidth()
            .height(fieldHeight)
            .border(borderWidth, borderColor, shape)
            .background(containerColor, shape)
            .padding(horizontal = horizontalPadding),
        ) {
          val hasSuffix = suffix != null
          val showStatusIcon = hasError || success

          Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
              .fillMaxWidth()
              .align(if (isInternal) Alignment.BottomCenter else Alignment.Center)
              .then(if (isInternal) Modifier.padding(bottom = verticalPadding) else Modifier)
              .then(if (showStatusIcon && !hasSuffix) Modifier.padding(end = 28.dp) else Modifier),
          ) {
            if (leadingIcon != null) {
              leadingIcon()
              Spacer(Modifier.width(10.dp))
            }

            Box(modifier = Modifier.weight(1f)) {
              if (isInternal) {
                val scale = 1f - (labelProgress * 0.23f)
                Text(
                  label,
                  style = if (labelActive) AppTheme.typography.action else AppTheme.typography.body,
                  color = if (labelActive) labelColor else AppTheme.colors.textMuted,
                  modifier = Modifier.graphicsLayer {
                    val fieldHeightPx = fieldHeight.toPx()
                    val paddingPx = verticalPadding.toPx()
                    val contentCenterY = fieldHeightPx - paddingPx - size.height / 2
                    val boxCenterY = fieldHeightPx / 2
                    val topTargetY = labelTopPadding.toPx() + size.height * scale / 2
                    val centerOffset = -(contentCenterY - boxCenterY)
                    val topOffset = -(contentCenterY - topTargetY)
                    scaleX = scale
                    scaleY = scale
                    translationY = centerOffset + labelProgress * (topOffset - centerOffset)
                    transformOrigin = TransformOrigin(0f, 0.5f)
                  },
                )
              }

              val showPlaceholder =
                value.isEmpty() && placeholder != null && (!isInternal || labelActive)
              if (showPlaceholder) {
                val placeholderAlpha by animateFloatAsState(
                  if (isInternal && !isFocused) 0f else 1f,
                  tween(150),
                )
                Text(
                  placeholder,
                  style = AppTheme.typography.body,
                  color = AppTheme.colors.textMuted,
                  modifier = Modifier.alpha(placeholderAlpha),
                )
              }
              innerTextField()
            }

            if (suffix != null) {
              val suffixVisible = !isInternal || value.isNotEmpty() || isFocused
              val suffixAlpha by animateFloatAsState(
                if (suffixVisible) 1f else 0f,
                tween(150),
              )

              if (suffixAlpha > 0f) {
                Spacer(Modifier.width(4.dp))
                Box(Modifier.alpha(suffixAlpha)) {
                  suffix()
                }
              }
            }

            // External label + suffix: icon inline after suffix
            if (!isInternal && hasSuffix && showStatusIcon) {
              Spacer(Modifier.width(8.dp))
              if (hasError) {
                Icon(
                  icon = Lucide.CircleAlert,
                  modifier = Modifier.size(18.dp),
                  tint = AppTheme.colors.danger,
                  contentDescription = "오류",
                )
              } else {
                Icon(
                  icon = Lucide.Check,
                  modifier = Modifier.size(18.dp),
                  tint = AppTheme.colors.success,
                  strokeWidth = 2.5f,
                  contentDescription = "확인됨",
                )
              }
            }
          }

          // No suffix: original absolute positioning
          if (!hasSuffix && hasError) {
            Icon(
              icon = Lucide.CircleAlert,
              modifier = Modifier.size(18.dp).align(Alignment.CenterEnd),
              tint = AppTheme.colors.danger,
              contentDescription = "오류",
            )
          } else if (!hasSuffix && success) {
            Icon(
              icon = Lucide.Check,
              modifier = Modifier.size(18.dp).align(Alignment.CenterEnd),
              tint = AppTheme.colors.success,
              strokeWidth = 2.5f,
              contentDescription = "확인됨",
            )
          }

          // Internal label + suffix: icon at top-end (above suffix)
          if (isInternal && hasSuffix && hasError) {
            Icon(
              icon = Lucide.CircleAlert,
              modifier = Modifier
                .align(Alignment.TopEnd)
                .offset(y = labelTopPadding)
                .size(18.dp),
              tint = AppTheme.colors.danger,
              contentDescription = "오류",
            )
          } else if (isInternal && hasSuffix && success) {
            Icon(
              icon = Lucide.Check,
              modifier = Modifier
                .align(Alignment.TopEnd)
                .offset(y = labelTopPadding)
                .size(18.dp),
              tint = AppTheme.colors.success,
              strokeWidth = 2.5f,
              contentDescription = "확인됨",
            )
          }
        }
      },
    )

    Spacer(Modifier.height(4.dp))

    val helpColor by animateColorAsState(
      if (hasError) AppTheme.colors.danger else AppTheme.colors.textTertiary,
      colorSpec,
    )

    Text(
      text = error ?: help ?: "",
      style = helpTextStyle,
      color = helpColor,
      modifier = Modifier
        .defaultMinSize(minHeight = 16.dp)
        .padding(start = 8.dp),
    )
  }
}

@Composable
fun TextField(
  field: FieldState<String>,
  label: String,
  modifier: Modifier = Modifier,
  help: String? = null,
  helpTextStyle: TextStyle = AppTheme.typography.micro,
  success: Boolean = false,
  placeholder: String? = null,
  labelPosition: LabelPosition = LabelPosition.External,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  autoFocus: Boolean = false,
  isPassword: Boolean = false,
  contentType: ContentType? = null,
  keyboardType: KeyboardType = KeyboardType.Text,
  imeAction: ImeAction? = null,
  onImeAction: (() -> Unit)? = null,
  leadingIcon: @Composable (() -> Unit)? = null,
  suffix: @Composable (() -> Unit)? = null,
) {
  val form = field.form
  val isSkeleton = LocalSkeleton.current.enabled
  val shouldAutoFocus = autoFocus || (form != null && form.isFirstField(field))

  val resolvedImeAction = imeAction ?: form?.imeActionFor(field)

  val resolvedOnImeAction: (() -> Unit)? = when {
    onImeAction != null -> onImeAction
    form != null -> {
      { form.focusNext(field) }
    }

    else -> null
  }

  if (shouldAutoFocus && !isSkeleton) {
    LaunchedEffect(shouldAutoFocus) {
      field.focusRequester.requestFocus()
    }
  }

  TextField(
    value = field.value,
    onValueChange = { field.setValue(it) },
    label = label,
    modifier = if (!isSkeleton && (form != null || autoFocus)) {
      modifier.focusRequester(field.focusRequester)
    } else {
      modifier
    },
    help = help,
    helpTextStyle = helpTextStyle,
    error = field.errors.firstOrNull(),
    success = success,
    placeholder = placeholder,
    labelPosition = labelPosition,
    enabled = enabled,
    readOnly = readOnly,
    autoFocus = false,
    isPassword = isPassword,
    contentType = contentType,
    onBlur = { field.onBlur() },
    keyboardType = keyboardType,
    imeAction = resolvedImeAction,
    onImeAction = resolvedOnImeAction,
    leadingIcon = leadingIcon,
    suffix = suffix,
  )
}
