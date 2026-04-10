package co.typie.screen.home

import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

private val SearchScreenHorizontalPadding = 20.dp

@Composable
fun SearchHeader(
  animateOnEnter: Boolean,
  placeholder: String,
  query: String,
  onQueryChange: (String) -> Unit,
  onSubmit: () -> Unit,
  onEnterAnimationConsumed: () -> Unit,
) {
  val focusRequester = remember { FocusRequester() }
  var shouldShow by remember { mutableStateOf(!animateOnEnter) }
  var isFocused by remember { mutableStateOf(false) }
  var textFieldValue by remember { mutableStateOf(TextFieldValue(query, TextRange(query.length))) }
  val containerAlpha by animateFloatAsState(
    targetValue = if (shouldShow) 1f else 0f,
    animationSpec = tween(200),
    label = "search-header-alpha",
  )
  val containerOffsetY by animateDpAsState(
    targetValue = if (shouldShow) 0.dp else 12.dp,
    animationSpec = tween(200, easing = EaseOut),
    label = "search-header-offset",
  )

  LaunchedEffect(query) {
    if (textFieldValue.text != query) {
      textFieldValue = TextFieldValue(query, TextRange(query.length))
    }
  }

  LaunchedEffect(animateOnEnter) {
    if (animateOnEnter) {
      shouldShow = true
      focusRequester.requestFocus()
      onEnterAnimationConsumed()
    } else {
      shouldShow = true
    }
  }

  Box(
    modifier = Modifier
      .fillMaxWidth()
      .offset(y = containerOffsetY)
      .alpha(containerAlpha),
  ) {
    BasicTextField(
      value = textFieldValue,
      onValueChange = {
        textFieldValue = it
        onQueryChange(it.text)
      },
      singleLine = true,
      textStyle = AppTheme.typography.body.copy(color = AppTheme.colors.textPrimary),
      cursorBrush = SolidColor(AppTheme.colors.textPrimary),
      keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
      keyboardActions = KeyboardActions(onSearch = { onSubmit() }),
      modifier = Modifier
        .padding(horizontal = SearchScreenHorizontalPadding)
        .padding(bottom = 4.dp)
        .fillMaxWidth()
        .focusRequester(focusRequester)
        .onFocusChanged { isFocused = it.isFocused },
      decorationBox = { innerTextField ->
        HomeSearchFieldFrame(
          focused = isFocused,
          modifier = Modifier.fillMaxWidth(),
        ) {
          Icon(
            icon = Lucide.Search,
            modifier = Modifier.size(HomeSearchFieldDefaults.IconSize),
            tint = AppTheme.colors.textMuted,
          )

          Spacer(Modifier.width(HomeSearchFieldDefaults.IconGap))

          Box(Modifier.weight(1f)) {
            if (textFieldValue.text.isEmpty()) {
              Text(
                placeholder,
                style = AppTheme.typography.body,
                color = AppTheme.colors.textMuted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }
            innerTextField()
          }

          androidx.compose.animation.AnimatedVisibility(
            visible = textFieldValue.text.isNotEmpty(),
            enter = androidx.compose.animation.fadeIn(tween(150)) + scaleIn(initialScale = 0.8f, animationSpec = tween(150)),
            exit = androidx.compose.animation.fadeOut(tween(150)) + scaleOut(targetScale = 0.8f, animationSpec = tween(150)),
          ) {
            Spacer(Modifier.width(HomeSearchFieldDefaults.ClearIconGap))
            Icon(
              icon = Lucide.CircleX,
              modifier = Modifier.size(HomeSearchFieldDefaults.IconSize).clickable { onQueryChange("") },
              tint = AppTheme.colors.textMuted,
            )
          }
        }
      },
    )
  }
}
