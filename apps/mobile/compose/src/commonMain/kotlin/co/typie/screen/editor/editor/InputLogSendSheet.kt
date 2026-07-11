package co.typie.screen.editor.editor

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Button
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.theme.AppTheme

@Composable
context(_: SheetScope<String>)
internal fun InputLogSendSheet() {
  var name by remember { mutableStateOf("") }

  SheetLayout(footer = { Button(text = "보내기", onClick = { complete(name.trim()) }) }) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
      verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
      Text(
        text = "입력 기록 전송",
        style = AppTheme.typography.title,
        color = AppTheme.colors.textDefault,
      )
      Text(
        text = "이 글의 최근 입력 기록이 개발자에게 분석 목적으로 전송돼요. 제품 개선 외의 목적으로는 사용되지 않아요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
      )
      TextField(
        value = name,
        onValueChange = { name = it },
        label = "설명",
        placeholder = "설명을 입력하세요",
        autoFocus = true,
      )
    }
  }
}
