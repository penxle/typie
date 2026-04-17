package co.typie.screen.settings.aisettings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.settings.SettingControlRow
import co.typie.domain.settings.SettingSwitch
import co.typie.ext.verticalScroll
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun AiSettingsScreen() {
  val model = viewModel { AiSettingsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val dialog = LocalDialog.current
  val toast = LocalToast.current

  fun update(enabled: Boolean) {
    scope.launch {
      if (enabled) {
        val result =
          dialog.alert(
            title = "AI 기능을 활성화하시겠어요?",
            message = "사용자의 글은 AI 모델 학습에 절대 사용되지 않으며, 사용자가 요청할 때만 AI가 사용돼요. 언제든지 설정에서 비활성화할 수 있어요.",
            confirmText = "활성화",
          )

        if (result is DialogResult.Dismissed) {
          return@launch
        }
      }

      model.updateAiOptIn(enabled).withDefaultExceptionHandler(toast)
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("AI", style = AppTheme.typography.title) },
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(20.dp),
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Text("타이피는 사용자의 글을 절대 학습하지 않아요", style = AppTheme.typography.title)

          Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
              "타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은 어떠한 경우에도 AI 모델 학습에 사용되지 않아요.",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textTertiary,
            )

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
              NoticeItem(title = "학습 금지", description = "사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.")
              NoticeItem(title = "요청 시에만", description = "사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.")
              NoticeItem(title = "투명한 처리", description = "AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.")
              NoticeItem(
                title = "완전한 통제",
                description = "AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.",
              )
              NoticeItem(title = "권리 보장", description = "타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.")
            }
          }
        }
      }

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        SettingControlRow(
          label = "AI 기능 활성화",
          description = "활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.",
          enabled = !model.isSubmitting,
          onClick = { update(!model.aiOptIn) },
          trailing = {
            SettingSwitch(checked = model.aiOptIn, onCheckedChange = { next -> update(next) })
          },
        )
      }
    }
  }
}

@Composable
private fun NoticeItem(title: String, description: String) {
  Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
    Text("•", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)

    Text(
      buildAnnotatedString {
        withStyle(SpanStyle(fontWeight = FontWeight.W600)) { append("$title: ") }
        append(description)
      },
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.weight(1f),
    )
  }
}
