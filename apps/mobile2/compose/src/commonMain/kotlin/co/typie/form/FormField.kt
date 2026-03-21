package co.typie.form

import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun <V> FormField(
  field: FieldState<V>,
  content: @Composable (FieldState<V>) -> Unit,
) {
  var wasFocused by remember { mutableStateOf(false) }
  Column(
    modifier = Modifier.onFocusChanged { state ->
      if (wasFocused && !state.hasFocus) field.onBlur()
      wasFocused = state.hasFocus
    },
  ) {
    content(field)
    if (field.errors.isNotEmpty()) {
      Text(
        text = field.errors.first(),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textDanger,
      )
    }
  }
}
