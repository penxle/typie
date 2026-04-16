package co.typie.screen.more.feedback

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextArea
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private data class FeedbackTopic(val value: String, val label: String)

private data class FeedbackMood(val value: String, val icon: IconData)

private val FeedbackTopics =
  listOf(
    FeedbackTopic(value = "editor", label = "글쓰기/편집"),
    FeedbackTopic(value = "share", label = "발행/공유"),
    FeedbackTopic(value = "design", label = "테마/디자인"),
    FeedbackTopic(value = "billing", label = "구독/결제"),
    FeedbackTopic(value = "other", label = "기타"),
  )

private val FeedbackMoods =
  listOf(
    FeedbackMood(value = "angry", icon = Lucide.Angry),
    FeedbackMood(value = "annoyed", icon = Lucide.Annoyed),
    FeedbackMood(value = "good", icon = Lucide.Smile),
    FeedbackMood(value = "great", icon = Lucide.Laugh),
  )

@Composable
fun FeedbackScreen() {
  val model = viewModel { FeedbackViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  ProvideTopBar(
    center = { Text("의견 보내기", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        FeedbackTopics.forEach { item ->
          FeedbackTopicChip(
            label = item.label,
            selected = model.form.topic.value == item.value,
            onClick = { model.form.topic.value = item.value },
          )
        }
      }

      TextArea(field = model.form.content, placeholder = "칭찬도, 불만도, 아이디어도 다 좋아요!")

      Row(
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        for (mood in FeedbackMoods) {
          FeedbackMoodButton(
            icon = mood.icon,
            selected = model.form.mood.value == mood.value,
            onClick = {
              model.form.mood.value = if (model.form.mood.value == mood.value) null else mood.value
            },
          )
        }
      }

      Button(
        text = "보내기",
        loading = model.isSubmitting,
        loadingText = "보내는 중...",
        onClick = {
          model.submit().withDefaultExceptionHandler(toast).onOk {
            toast.success("피드백을 보냈어요. 감사해요!")
            nav.pop()
          }
        },
      )
    }
  }
}

@Composable
private fun FeedbackTopicChip(label: String, selected: Boolean, onClick: suspend () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.border(
            width = 1.dp,
            color = if (selected) AppTheme.colors.brand else AppTheme.colors.borderDefault,
            shape = AppShapes.circle,
          )
          .background(
            color = if (selected) AppTheme.colors.brandSubtle else AppTheme.colors.surfaceBase,
            shape = AppShapes.circle,
          )
          .clickable(onClick)
          .padding(horizontal = 12.dp, vertical = 8.dp)
          .pressScale(),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = label,
        style = AppTheme.typography.action,
        color = if (selected) AppTheme.colors.brand else AppTheme.colors.textSecondary,
      )
    }
  }
}

@Composable
private fun FeedbackMoodButton(icon: IconData, selected: Boolean, onClick: suspend () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.size(34.dp)
          .border(
            width = 1.dp,
            color = if (selected) AppTheme.colors.brand else AppTheme.colors.borderDefault,
            shape = AppShapes.circle,
          )
          .background(
            color = if (selected) AppTheme.colors.brandSubtle else AppTheme.colors.surfaceDefault,
            shape = AppShapes.circle,
          )
          .clickable(onClick)
          .pressScale(),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = icon,
        modifier = Modifier.size(18.dp),
        tint = if (selected) AppTheme.colors.brand else AppTheme.colors.textTertiary,
      )
    }
  }
}
