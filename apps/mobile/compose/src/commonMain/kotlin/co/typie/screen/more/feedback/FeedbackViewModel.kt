package co.typie.screen.more.feedback

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.graphql.Apollo
import co.typie.graphql.FeedbackScreen_SubmitFeedback_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.SubmitFeedbackInput
import co.typie.platform.PlatformModule
import co.typie.result.Result
import co.typie.result.loading
import com.apollographql.apollo.api.Optional
import kotlinx.coroutines.CoroutineScope

sealed interface FeedbackError {
  data class ValidationFailed(val errorMessage: String) : FeedbackError
}

class FeedbackForm(scope: CoroutineScope) : FormState(scope) {
  val topic =
    field<String?>(null) {
      required("주제를 선택해주세요.")
      focusable = false
    }
  val mood = field<String?>(null) { focusable = false }
  val content = field("") { required("내용을 입력해주세요.") }
}

class FeedbackViewModel : ViewModel() {
  val form = FeedbackForm(viewModelScope)

  var isSubmitting by mutableStateOf(false)
    private set

  suspend fun submit(): Result<Unit, FeedbackError> {
    if (!form.validate()) return Result.Err(FeedbackError.ValidationFailed(form.errorMessage!!))

    return loading({ isSubmitting = it }) {
      val deviceInfo = PlatformModule.deviceInfo.retrieve()

      Apollo.executeMutation(
        FeedbackScreen_SubmitFeedback_Mutation(
          input =
            SubmitFeedbackInput(
              topic = Optional.presentIfNotNull(form.topic.value),
              content = form.content.value.trim(),
              mood = Optional.presentIfNotNull(form.mood.value),
              platform = Optional.present(deviceInfo.osName),
              osVersion = Optional.present(deviceInfo.osVersion),
              appVersion =
                Optional.present("${deviceInfo.appVersion} (${deviceInfo.appBuildNumber})"),
            )
        )
      )
    }
  }
}
