package co.typie.ui.component

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
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
import co.typie.form.FieldState
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

enum class LabelPosition {
  External,
  Internal,
}

@Composable
fun TextField(
  value: String,
  onValueChange: (String) -> Unit,
  label: String,
  modifier: Modifier = Modifier,
  help: String? = null,
  error: String? = null,
  success: Boolean = false,
  placeholder: String? = null,
  labelPosition: LabelPosition = LabelPosition.External,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  isPassword: Boolean = false,
  onBlur: (() -> Unit)? = null,
  keyboardType: KeyboardType = KeyboardType.Text,
  imeAction: ImeAction? = null,
  onImeAction: (() -> Unit)? = null,
  leadingIcon: @Composable (() -> Unit)? = null,
) {
  val shape = RoundedCornerShape(12.dp)
  var isFocused by remember { mutableStateOf(false) }
  val hasError = error != null
  val isInternal = labelPosition == LabelPosition.Internal
  val resolvedImeAction = imeAction ?: ImeAction.Default

  val colorSpec = tween<androidx.compose.ui.graphics.Color>(220)

  val containerColor by animateColorAsState(
    when {
      !enabled -> AppTheme.colors.surfaceMuted
      isFocused -> AppTheme.colors.surfaceDefault
      else -> AppTheme.colors.surfaceSubtle
    },
    colorSpec,
  )

  val borderColor by animateColorAsState(
    when {
      hasError -> AppTheme.colors.accentDangerSubtle
      isFocused -> AppTheme.colors.borderInverse
      else -> AppTheme.colors.borderSubtle
    },
    colorSpec,
  )

  val borderWidth by animateDpAsState(
    if (isFocused || hasError) 1.5.dp else 1.dp,
    spring(stiffness = Spring.StiffnessMediumLow),
  )

  val horizontalPadding = 16.dp

  val labelColor by animateColorAsState(
    when {
      hasError -> AppTheme.colors.accentDanger
      isFocused -> AppTheme.colors.textDefault
      else -> AppTheme.colors.textSubtle
    },
    colorSpec,
  )

  val labelActive = isInternal && (isFocused || value.isNotEmpty())
  val fieldHeight = if (isInternal) 56.dp else 48.dp

  val labelProgress by animateFloatAsState(
    if (labelActive) 1f else 0f,
    spring(dampingRatio = Spring.DampingRatioLowBouncy, stiffness = Spring.StiffnessMediumLow),
  )

  Column(modifier = modifier) {
    if (!isInternal) {
      Text(
        label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textSubtle,
      )

      Spacer(Modifier.height(8.dp))
    }

    BasicTextField(
      value = value,
      onValueChange = onValueChange,
      enabled = enabled,
      readOnly = readOnly,
      modifier = Modifier
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
        color = if (enabled) AppTheme.colors.textDefault else AppTheme.colors.textDisabled,
      ),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      keyboardOptions = KeyboardOptions(
        keyboardType = keyboardType,
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
          Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
              .fillMaxWidth()
              .align(if (isInternal) Alignment.BottomCenter else Alignment.Center)
              .then(if (isInternal) Modifier.padding(bottom = 10.dp) else Modifier),
          ) {
            if (leadingIcon != null) {
              leadingIcon()
              Spacer(Modifier.width(10.dp))
            }

            Box(modifier = Modifier.weight(1f)) {
              if (isInternal) {
                val scale = 1f - (labelProgress * 0.23f)
                val translationY = labelProgress * -20f
                Text(
                  label,
                  style = AppTheme.typography.body,
                  color = if (labelActive) labelColor else AppTheme.colors.textDisabled,
                  modifier = Modifier.graphicsLayer {
                    scaleX = scale
                    scaleY = scale
                    this.translationY = translationY
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
                  color = AppTheme.colors.textDisabled,
                  modifier = Modifier.alpha(placeholderAlpha),
                )
              }
              innerTextField()
            }

            if (hasError) {
              Spacer(Modifier.width(10.dp))
              Icon(
                icon = Lucide.CircleAlert,
                modifier = Modifier.size(18.dp),
                tint = AppTheme.colors.accentDangerSubtle,
                contentDescription = "오류",
              )
            } else if (success) {
              Spacer(Modifier.width(10.dp))
              Icon(
                icon = Lucide.Check,
                modifier = Modifier.size(18.dp),
                tint = AppTheme.colors.accentSuccess,
                strokeWidth = 2.5f,
                contentDescription = "확인됨",
              )
            }
          }
        }
      },
    )

    Spacer(Modifier.height(4.dp))

    val helpColor by animateColorAsState(
      if (hasError) AppTheme.colors.accentDangerSubtle else AppTheme.colors.textMuted,
      colorSpec,
    )

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
fun TextField(
  field: FieldState<String>,
  label: String,
  modifier: Modifier = Modifier,
  help: String? = null,
  success: Boolean = false,
  placeholder: String? = null,
  labelPosition: LabelPosition = LabelPosition.External,
  enabled: Boolean = true,
  readOnly: Boolean = false,
  isPassword: Boolean = false,
  keyboardType: KeyboardType = KeyboardType.Text,
  imeAction: ImeAction? = null,
  onImeAction: (() -> Unit)? = null,
  leadingIcon: @Composable (() -> Unit)? = null,
) {
  val form = field.form

  val resolvedImeAction = imeAction ?: form?.imeActionFor(field)

  val resolvedOnImeAction: (() -> Unit)? = when {
    onImeAction != null -> onImeAction
    form != null -> {
      { form.focusNext(field) }
    }

    else -> null
  }

  if (form != null && form.isFirstField(field)) {
    LaunchedEffect(Unit) {
      field.focusRequester.requestFocus()
    }
  }

  TextField(
    value = field.value,
    onValueChange = { field.setValue(it) },
    label = label,
    modifier = if (form != null) {
      modifier.focusRequester(field.focusRequester)
    } else {
      modifier
    },
    help = help,
    error = field.errors.firstOrNull(),
    success = success,
    placeholder = placeholder,
    labelPosition = labelPosition,
    enabled = enabled,
    readOnly = readOnly,
    isPassword = isPassword,
    onBlur = { field.onBlur() },
    keyboardType = keyboardType,
    imeAction = resolvedImeAction,
    onImeAction = resolvedOnImeAction,
    leadingIcon = leadingIcon,
  )
}
