package co.typie.screen.editor.editor.aifeedback

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.Button
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
context(_: SheetScope<Boolean>)
internal fun AiFeedbackOptInSheet() {
  SheetLayout(footer = { Button(text = "AI 설정으로 이동", onClick = { complete(true) }) }) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
      Box(
        modifier =
          Modifier.size(56.dp)
            .clip(AppShapes.circle)
            .background(AppTheme.colors.palette.purple.copy(alpha = 0.14f)),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = Lucide.Lightbulb,
          modifier = Modifier.size(26.dp),
          tint = AppTheme.colors.palette.purple,
        )
      }
      Text(
        text = "AI 기능을 사용하려면\n설정에서 활성화해주세요",
        style = AppTheme.typography.title,
        color = AppTheme.colors.textDefault,
        textAlign = TextAlign.Center,
      )
    }
  }
}
