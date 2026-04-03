package co.typie.screen.more

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.MoreScreen_SubmitFeedback_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.SubmitFeedbackInput
import co.typie.icons.Lucide
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.DeviceInfo
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Text
import co.typie.ui.component.TextArea
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Optional
import kotlinx.coroutines.CancellationException
import org.koin.compose.koinInject

private data class FeedbackTopic(
  val value: String,
  val label: String,
)

private data class FeedbackMood(
  val value: String,
  val icon: IconData,
)

private val feedbackTopics = listOf(
  FeedbackTopic(value = "editor", label = "글쓰기/편집"),
  FeedbackTopic(value = "share", label = "발행/공유"),
  FeedbackTopic(value = "design", label = "테마/디자인"),
  FeedbackTopic(value = "billing", label = "구독/결제"),
  FeedbackTopic(value = "other", label = "기타"),
)

private val feedbackMoods = listOf(
  FeedbackMood(value = "angry", icon = Lucide.Angry),
  FeedbackMood(value = "annoyed", icon = Lucide.Annoyed),
  FeedbackMood(value = "good", icon = Lucide.Smile),
  FeedbackMood(value = "great", icon = Lucide.Laugh),
)

private data class FeedbackMetadata(
  val platform: String? = null,
  val osVersion: String? = null,
  val appVersion: String? = null,
  val deviceName: String? = null,
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun BottomSheetScope<Unit>.FeedbackSheet() {
  val apolloClient = koinInject<ApolloClient>()
  val deviceInfo = koinInject<DeviceInfo>()
  val toast = koinInject<Toast>()

  var topic by remember { mutableStateOf<String?>(null) }
  var mood by remember { mutableStateOf<String?>(null) }
  var content by remember { mutableStateOf("") }
  var isSubmitting by remember { mutableStateOf(false) }

  suspend fun submit() {
    if (isSubmitting) return

    if (topic == null) {
      toast.show(ToastType.Error, "주제를 선택해주세요.")
      return
    }

    val trimmedContent = content.trim()
    if (trimmedContent.isEmpty()) {
      toast.show(ToastType.Error, "내용을 입력해주세요.")
      return
    }

    isSubmitting = true

    try {
      val deviceSnapshot = runCatching {
        deviceInfo.snapshot()
      }.getOrElse {
        null
      }
      val metadata = buildFeedbackMetadata(deviceSnapshot)

      apolloClient.executeMutation(
        MoreScreen_SubmitFeedback_Mutation(
          input = SubmitFeedbackInput(
            topic = topic.toOptionalInput(),
            content = trimmedContent,
            mood = mood.toOptionalInput(),
            platform = metadata.platform.toOptionalInput(),
            osVersion = metadata.osVersion.toOptionalInput(),
            appVersion = metadata.appVersion.toOptionalInput(),
            deviceName = metadata.deviceName.toOptionalInput(),
          ),
        ),
      )

      toast.show(ToastType.Success, "피드백을 보냈어요. 감사해요!")
      dismiss()
    } catch (e: Exception) {
      if (e is CancellationException) {
        throw e
      }

      Logger.e(e) { "Failed to submit feedback" }
      toast.show(ToastType.Error, "의견 전송에 실패했어요. 잠시 후 다시 시도해주세요.")
    } finally {
      isSubmitting = false
    }
  }

  BottomSheetScaffold(title = "의견 보내기") {
    FlowRow(
      horizontalArrangement = Arrangement.spacedBy(8.dp),
      verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
      feedbackTopics.forEach { item ->
        FeedbackTopicChip(
          label = item.label,
          selected = topic == item.value,
          onClick = { topic = item.value },
        )
      }
    }

    TextArea(
      value = content,
      onValueChange = { content = it },
      placeholder = "칭찬도, 불만도, 아이디어도 다 좋아요!",
    )

    Row(
      horizontalArrangement = Arrangement.spacedBy(4.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      feedbackMoods.forEach { item ->
        FeedbackMoodButton(
          icon = item.icon,
          selected = mood == item.value,
          onClick = {
            mood = if (mood == item.value) null else item.value
          },
        )
      }
    }

    Button(
      text = "보내기",
      loadingText = "보내는 중...",
      variant = if (topic != null && content.trim().isNotEmpty()) {
        ButtonVariant.Primary
      } else {
        ButtonVariant.Secondary
      },
      loading = isSubmitting,
      enabled = !isSubmitting,
      onClick = { submit() },
    )
  }
}

private fun String?.toOptionalInput(): Optional<String> {
  val value = this?.trim()?.takeIf { it.isNotEmpty() } ?: return Optional.Absent
  return Optional.present(value)
}

private fun buildFeedbackMetadata(info: co.typie.platform.DeviceInfoSnapshot?): FeedbackMetadata {
  return FeedbackMetadata(
    platform = info?.platform?.trim()?.takeIf { it.isNotEmpty() },
    osVersion = info?.osVersion?.trim()?.takeIf { it.isNotEmpty() },
    appVersion = info?.appVersion?.trim()?.takeIf { it.isNotEmpty() },
    deviceName = info?.deviceName?.trim()?.takeIf { it.isNotEmpty() },
  )
}

@Composable
private fun FeedbackTopicChip(
  label: String,
  selected: Boolean,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = Modifier
        .border(
          width = 1.dp,
          color = if (selected) AppTheme.colors.brand else AppTheme.colors.borderDefault,
          shape = CircleShape,
        )
        .background(
          color = if (selected) AppTheme.colors.brandSubtle else AppTheme.colors.surfaceBase,
          shape = CircleShape,
        )
        .clickable(onClick)
        .padding(horizontal = 12.dp, vertical = 9.dp)
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
private fun FeedbackMoodButton(
  icon: IconData,
  selected: Boolean,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = Modifier
        .size(34.dp)
        .border(
          width = 1.dp,
          color = if (selected) AppTheme.colors.brand else AppTheme.colors.borderDefault,
          shape = CircleShape,
        )
        .background(
          color = if (selected) AppTheme.colors.brandSubtle else AppTheme.colors.surfaceDefault,
          shape = CircleShape,
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
