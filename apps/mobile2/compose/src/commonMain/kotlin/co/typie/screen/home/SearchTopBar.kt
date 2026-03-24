package co.typie.screen.home

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

val SearchTopBarKey = Any()

@Composable
fun SearchTopBar(
  query: String,
  onQueryChange: (String) -> Unit,
  onSubmit: () -> Unit,
  onCancel: () -> Unit,
) {
  val focusRequester = remember { FocusRequester() }
  var textFieldValue by remember { mutableStateOf(TextFieldValue(query, TextRange(query.length))) }

  LaunchedEffect(query) {
    if (textFieldValue.text != query) {
      textFieldValue = TextFieldValue(query, TextRange(query.length))
    }
  }

  LaunchedEffect(Unit) {
    focusRequester.requestFocus()
  }

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.fillMaxWidth().height(TopBarDefaults.Height),
  ) {
    Row(
      verticalAlignment = Alignment.CenterVertically,
      modifier = Modifier
        .weight(1f)
        .height(36.dp)
        .background(AppTheme.colors.surfaceDefault, RoundedCornerShape(10.dp))
        .padding(horizontal = 12.dp),
    ) {
      Icon(
        icon = Lucide.Search,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textMuted,
      )

      Spacer(Modifier.width(8.dp))

      BasicTextField(
        value = textFieldValue,
        onValueChange = {
          textFieldValue = it
          onQueryChange(it.text)
        },
        singleLine = true,
        textStyle = AppTheme.typography.action.copy(color = AppTheme.colors.textPrimary),
        cursorBrush = SolidColor(AppTheme.colors.brand),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { onSubmit() }),
        modifier = Modifier.weight(1f).focusRequester(focusRequester),
        decorationBox = { innerTextField ->
          if (textFieldValue.text.isEmpty()) {
            Text(
              "문서 검색...",
              style = AppTheme.typography.action,
              color = AppTheme.colors.textMuted,
            )
          }
          innerTextField()
        },
      )

      AnimatedVisibility(
        visible = query.isNotEmpty(),
        enter = fadeIn(tween(150)) + scaleIn(initialScale = 0.8f, animationSpec = tween(150)),
        exit = fadeOut(tween(150)) + scaleOut(targetScale = 0.8f, animationSpec = tween(150)),
      ) {
        Row {
          Spacer(Modifier.width(8.dp))
          Icon(
            icon = Lucide.CircleX,
            modifier = Modifier.size(16.dp).clickable { onQueryChange("") },
            tint = AppTheme.colors.textMuted,
          )
        }
      }
    }

    Spacer(Modifier.width(12.dp))

    Text(
      "취소",
      style = AppTheme.typography.action,
      color = AppTheme.colors.brand,
      modifier = Modifier.clickable(onClick = onCancel),
    )
  }
}
