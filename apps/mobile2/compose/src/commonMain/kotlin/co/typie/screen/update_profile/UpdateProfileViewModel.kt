package co.typie.screen.update_profile

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.blob.BlobService
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.maxLength
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.UpdateProfileScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.UpdateProfileScreen_Query
import co.typie.graphql.UpdateProfileScreen_UpdateUser_Mutation
import co.typie.graphql.text
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateUserInput
import co.typie.graphql.type.buildUser
import co.typie.platform.PlatformFile
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.state.AsyncAction
import kotlinx.coroutines.CoroutineScope
import org.koin.core.annotation.KoinViewModel

class UpdateProfileForm(scope: CoroutineScope) : FormState(scope) {
  val name = field("") {
    required("닉네임을 입력해주세요.")
    validateOn(ValidateOn.Change) {
      maxLength(20, "닉네임은 20자를 넘을 수 없어요.")
    }
  }

  val avatarId = field("") {
    focusable = false
    required("프로필 사진을 선택해주세요.")
  }
}

class UpdateProfileScreenState(scope: CoroutineScope) {
  val form = UpdateProfileForm(scope)
  var avatarPreviewUrl: String? by mutableStateOf(null)
}

@KoinViewModel
class UpdateProfileViewModel(
  private val blobService: BlobService,
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = UpdateProfileScreenState(viewModelScope)
  val submitAction = AsyncAction(viewModelScope)

  val query =
    watchQuery(
      placeholderData = placeholderData(),
      onInitialData = { data ->
        state.form.name.initialValue = data.me.name
        state.form.avatarId.initialValue = data.me.avatar.id
      },
    ) { UpdateProfileScreen_Query() }

  suspend fun uploadAvatar(file: PlatformFile): String? {
    return try {
      toast.withLoading(
        message = "프로필 사진 업로드 중...",
        errorMessage = "프로필 사진 업로드에 실패했어요. 다시 시도해주세요.",
      ) {
        val path = blobService.uploadBytes(
          bytes = file.bytes,
          filename = file.filename,
          mimeType = file.mimeType,
        )

        val result = executeMutation(
          UpdateProfileScreen_PersistBlobAsImage_Mutation(
            input = PersistBlobAsImageInput(path = path),
          ),
        )

        state.avatarPreviewUrl = result.persistBlobAsImage.url
        success("프로필 사진이 업로드되었어요.")

        result.persistBlobAsImage.id
      }
    } catch (e: Exception) {
      Logger.e(e) { "Failed to upload avatar" }
      null
    }
  }

  fun submit(onSubmit: suspend () -> Unit) {
    submitAction.launch(
      onFailure = { e ->
        Logger.e(e) { "Failed to update profile" }
        toast.show(ToastType.Error, e.message ?: "프로필 변경에 실패했어요. 다시 시도해주세요.")
      },
    ) {
        if (!state.form.validate()) return@launch

        executeMutation(
          UpdateProfileScreen_UpdateUser_Mutation(
            UpdateUserInput(
              avatarId = state.form.avatarId.value,
              name = state.form.name.value.trim(),
            ),
          ),
        )

        toast.show(ToastType.Success, "프로필이 변경되었어요.")

        state.form.commit()
        onSubmit()
    }
  }
}

private fun placeholderData() = UpdateProfileScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
  }
}
