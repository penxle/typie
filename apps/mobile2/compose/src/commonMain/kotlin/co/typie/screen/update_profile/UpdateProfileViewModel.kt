package co.typie.screen.update_profile

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import co.typie.blob.BlobUploader
import co.typie.graphql.MutationResult
import co.typie.graphql.UpdateProfileScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.UpdateProfileScreen_UpdateUser_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateUserInput
import co.typie.media.PickedImage
import com.apollographql.apollo.ApolloClient

class UpdateProfileScreenState {
  var errorMessage: String? by mutableStateOf(null)
    private set

  var completedSubmissionCount: Int by mutableStateOf(0)
    private set

  var avatarPreviewUrl: String? by mutableStateOf(null)
    private set

  var isUploadingAvatar: Boolean by mutableStateOf(false)
    private set

  fun complete() {
    completedSubmissionCount += 1
  }

  fun fail(message: String) {
    errorMessage = message
  }

  fun clearError() {
    errorMessage = null
  }

  fun beginAvatarUpload() {
    isUploadingAvatar = true
  }

  fun finishAvatarUpload() {
    isUploadingAvatar = false
  }

  fun updateAvatarPreviewUrl(url: String) {
    avatarPreviewUrl = url
  }
}

class UpdateProfileViewModel(
  private val apolloClient: ApolloClient,
  private val blobUploader: BlobUploader,
) : ViewModel() {
  val state = UpdateProfileScreenState()

  suspend fun uploadAvatar(image: PickedImage): String? {
    state.beginAvatarUpload()

    return try {
      val path = blobUploader.uploadBytes(
        bytes = image.bytes,
        filename = image.filename,
        mimeType = image.mimeType,
      )

      when (
        val result = apolloClient.executeMutation(
          UpdateProfileScreen_PersistBlobAsImage_Mutation(
            input = PersistBlobAsImageInput(path = path),
          ),
        )
      ) {
        is MutationResult.Success -> {
          val avatar = result.data.persistBlobAsImage
          state.updateAvatarPreviewUrl(avatar.url)
          avatar.id
        }

        is MutationResult.Failure -> {
          state.fail(result.error.message ?: "프로필 사진 업로드에 실패했어요. 다시 시도해주세요.")
          null
        }

        is MutationResult.Error -> {
          state.fail("프로필 사진 업로드에 실패했어요. 다시 시도해주세요.")
          null
        }
      }
    } catch (_: Exception) {
      state.fail("프로필 사진 업로드에 실패했어요. 다시 시도해주세요.")
      null
    } finally {
      state.finishAvatarUpload()
    }
  }

  suspend fun submit(
    name: String,
    avatarId: String,
  ) {
    when (
      val result = apolloClient.executeMutation(
        UpdateProfileScreen_UpdateUser_Mutation(
          UpdateUserInput(
            avatarId = avatarId,
            name = name,
          ),
        ),
      )
    ) {
      is MutationResult.Success -> state.complete()
      is MutationResult.Failure -> {
        state.fail(result.error.message ?: "프로필 변경에 실패했어요. 다시 시도해주세요.")
      }

      is MutationResult.Error -> {
        state.fail("프로필 변경에 실패했어요. 다시 시도해주세요.")
      }
    }
  }

  fun consumeError() {
    state.clearError()
  }
}
