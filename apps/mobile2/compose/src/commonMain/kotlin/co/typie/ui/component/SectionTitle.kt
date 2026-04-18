package co.typie.ui.component

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.ui.theme.AppTheme

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
  Text(
    text = text,
    modifier = modifier,
    style = AppTheme.typography.title,
    color = AppTheme.colors.textMuted,
  )
}
