package co.typie.screen.editor.editor.subpane.comments

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.icons.Lucide
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
internal fun CommentEditTextArea(
  value: String,
  onValueChange: (String) -> Unit,
  onFocusChange: (Boolean) -> Unit,
) {
  CommentTextInput(
    value = value,
    onValueChange = onValueChange,
    placeholder = "코멘트 수정...",
    shape = RoundedCornerShape(8.dp),
    maxLines = 4,
    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 9.dp),
    autoFocus = true,
    onFocusChange = onFocusChange,
  )
}

@Composable
internal fun CommentComposer(
  value: String,
  onValueChange: (String) -> Unit,
  placeholder: String,
  submitting: Boolean,
  onFocusChange: (Boolean) -> Unit,
  onSubmit: (String) -> Unit,
) {
  val hasText = value.isNotBlank()
  val filled = hasText || submitting

  Box(modifier = Modifier.fillMaxWidth()) {
    CommentTextInput(
      value = value,
      onValueChange = onValueChange,
      placeholder = placeholder,
      shape = RoundedCornerShape(20.dp),
      maxLines = 5,
      contentPadding = PaddingValues(start = 14.dp, end = 44.dp, top = 10.dp, bottom = 10.dp),
      autoFocus = false,
      onFocusChange = onFocusChange,
    )

    Box(
      modifier =
        Modifier.align(Alignment.BottomEnd)
          .padding(end = 6.dp, bottom = 6.dp)
          .size(28.dp)
          .clip(CircleShape)
          .background(if (filled) AppTheme.colors.surfaceInverse else Color.Transparent)
          .clickable(enabled = hasText && !submitting) { onSubmit(value) },
      contentAlignment = Alignment.Center,
    ) {
      if (submitting) {
        Spinner(color = AppTheme.colors.textOnInverse, modifier = Modifier.size(16.dp))
      } else {
        Icon(
          icon = Lucide.ArrowUp,
          modifier = Modifier.size(16.dp),
          tint = if (hasText) AppTheme.colors.textOnInverse else AppTheme.colors.textHint,
        )
      }
    }
  }
}

@Composable
internal fun CommentTextActionButton(
  text: String,
  color: Color,
  loading: Boolean = false,
  onClick: () -> Unit,
) {
  InteractionScope {
    Box(
      modifier =
        Modifier.height(28.dp)
          .clip(RoundedCornerShape(6.dp))
          .clickable(enabled = !loading) { onClick() }
          .pressScale(0.95f)
          .padding(horizontal = 4.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = text,
        modifier = Modifier.graphicsLayer { alpha = if (loading) 0f else 1f },
        style = AppTheme.typography.action,
        color = color,
      )
      if (loading) {
        Spinner(color = color)
      }
    }
  }
}

@Composable
private fun CommentTextInput(
  value: String,
  onValueChange: (String) -> Unit,
  placeholder: String,
  shape: RoundedCornerShape,
  maxLines: Int,
  contentPadding: PaddingValues,
  autoFocus: Boolean,
  onFocusChange: (Boolean) -> Unit,
) {
  val focusManager = LocalFocusManager.current
  var isFocused by remember { mutableStateOf(false) }
  val latestIsFocused = rememberUpdatedState(isFocused)
  val latestOnFocusChange = rememberUpdatedState(onFocusChange)
  val inputState =
    rememberTextInputState(
      value = value,
      onValueChange = onValueChange,
      onDismiss = { focusManager.clearFocus() },
    )

  if (autoFocus) {
    LaunchedEffect(Unit) { inputState.requestFocus() }
  }

  DisposableEffect(Unit) {
    onDispose {
      if (latestIsFocused.value) {
        latestOnFocusChange.value(false)
      }
    }
  }

  BasicTextField(
    value = inputState.value,
    onValueChange = inputState::onValueChange,
    modifier =
      Modifier.fillMaxWidth().heightIn(min = 40.dp).textInputFocusable(inputState) { state ->
        isFocused = state.isFocused
        onFocusChange(state.isFocused)
      },
    textStyle = AppTheme.typography.body.copy(color = AppTheme.colors.textDefault),
    cursorBrush = SolidColor(AppTheme.colors.textDefault),
    keyboardOptions =
      KeyboardOptions(
        capitalization = KeyboardCapitalization.Sentences,
        imeAction = ImeAction.Default,
      ),
    minLines = 1,
    maxLines = maxLines,
    decorationBox = { innerTextField ->
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .border(
              width = if (isFocused) 1.5.dp else 1.dp,
              color =
                if (isFocused) AppTheme.colors.borderEmphasis else AppTheme.colors.borderHairline,
              shape = shape,
            )
            .background(
              if (isFocused) AppTheme.colors.surfaceDefault else AppTheme.colors.surfaceInset,
              shape,
            )
            .padding(contentPadding)
      ) {
        if (inputState.value.text.isEmpty()) {
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
}
