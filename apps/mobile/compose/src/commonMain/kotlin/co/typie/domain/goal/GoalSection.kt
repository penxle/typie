package co.typie.domain.goal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun GoalSection(label: String, content: @Composable () -> Unit) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(GoalSectionLabelGap),
  ) {
    Text(text = label, style = AppTheme.typography.micro, color = AppTheme.colors.textHint)

    content()
  }
}

private val GoalSectionLabelGap = 8.dp
