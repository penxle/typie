package co.typie.screen.settings.update_profile

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.blob.BlobService
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.maxLength
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.UpdateProfileScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.UpdateProfileScreen_Query
import co.typie.graphql.UpdateProfileScreen_UpdateUser_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateUserInput
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformFile
import co.typie.result.Result
import co.typie.result.Task
import co.typie.result.loading
import co.typie.result.task
import kotlinx.coroutines.CoroutineScope

class UpdateProfileForm(scope: CoroutineScope) : FormState(scope) {
  val name =
    field("") {
      required("닉네임을 입력해주세요.")
      validateOn(ValidateOn.Change) { maxLength(20, "닉네임은 20자를 넘을 수 없어요.") }
    }

  val avatarId =
    field("") {
      focusable = false
      required("프로필 사진을 선택해주세요.")
    }
}

class UpdateProfileScreenState(scope: CoroutineScope) {
  val form = UpdateProfileForm(scope)
  var avatarPreviewUrl: String? by mutableStateOf(null)
}

class UpdateProfileViewModel : ViewModel() {
  private val blobService = BlobService
  val state = UpdateProfileScreenState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = { data ->
        state.form.name.initialValue = data.me.name
        state.form.avatarId.initialValue = data.me.avatar.id
      },
    ) {
      UpdateProfileScreen_Query()
    }

  fun uploadAvatar(file: PlatformFile): Task<Unit, String, Nothing> = task {
    emit(Unit)

    val path =
      blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )

    val result =
      Apollo.executeMutation(
        UpdateProfileScreen_PersistBlobAsImage_Mutation(
          input = PersistBlobAsImageInput(path = path)
        )
      )

    state.avatarPreviewUrl = result.persistBlobAsImage.url
    result.persistBlobAsImage.id
  }

  suspend fun submit(): Result<Unit, Nothing> {
    if (!state.form.validate()) return Result.Ok(Unit)

    return loading({ isSubmitting = it }) {
      Apollo.executeMutation(
        UpdateProfileScreen_UpdateUser_Mutation(
          UpdateUserInput(avatarId = state.form.avatarId.value, name = state.form.name.value.trim())
        )
      )

      state.form.commit()
    }
  }
}

private fun placeholderData() =
  UpdateProfileScreen_Query.Data(PlaceholderResolver) { me = buildUser { name = text(3..6) } }
