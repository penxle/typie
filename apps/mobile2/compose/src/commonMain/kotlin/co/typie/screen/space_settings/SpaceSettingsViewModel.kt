package co.typie.screen.space_settings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.blob.BlobService
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.maxLength
import co.typie.form.minLength
import co.typie.form.pattern
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SpaceSettingsScreen_DeleteSite_Mutation
import co.typie.graphql.SpaceSettingsScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.SpaceSettingsScreen_Query
import co.typie.graphql.SpaceSettingsScreen_UpdateSiteSlug_Mutation
import co.typie.graphql.SpaceSettingsScreen_UpdateSite_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.text
import co.typie.graphql.type.DeleteSiteInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.UpdateSiteInput
import co.typie.graphql.type.UpdateSiteSlugInput
import co.typie.graphql.type.buildSite
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.PlatformFile
import co.typie.service.SiteService
import com.apollographql.apollo.api.Optional
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.apolloStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

private val UNAVAILABLE_SITE_SLUGS =
  listOf("admin", "app", "cname", "dev", "docs", "help", "template", "www")

class SpaceSettingsForm(scope: CoroutineScope) : FormState(scope) {
  val name = field("") {
    required("스페이스 이름을 입력해주세요.")
    validateOn(ValidateOn.Change) {
      minLength(1, "스페이스 이름을 입력해주세요.")
    }
  }

  val slug = field("") {
    required("스페이스 주소를 입력해주세요.")
    validateOn(ValidateOn.Change) {
      minLength(4, "스페이스 주소는 4글자 이상이여야 해요")
      maxLength(63, "스페이스 주소는 63글자를 넘을 수 없어요")
      pattern(Regex("^[\\da-z-]+$"), "스페이스 주소는 소문자, 숫자, 하이픈만 사용할 수 있어요")
      pattern(Regex("^(?!.*--)[\\da-z-]+$"), "하이픈을 연속으로 사용할 수 없어요")
      pattern(Regex("^[\\da-z][\\da-z-]*[\\da-z]$"), "스페이스 주소는 하이픈으로 시작하거나 끝날 수 없어요")
      rule { if (it in UNAVAILABLE_SITE_SLUGS) "사용할 수 없는 스페이스 주소에요" else null }
    }
  }

  val logoId = field("") {
    focusable = false
  }
}

class SpaceSettingsScreenState(scope: CoroutineScope) {
  val form = SpaceSettingsForm(scope)
  var logoPreviewUrl: String? by mutableStateOf(null)
  var isSubmitting by mutableStateOf(false)
  var isDeleting by mutableStateOf(false)
}

@KoinViewModel
class SpaceSettingsViewModel(
  val siteService: SiteService,
  private val blobService: BlobService,
  private val toast: Toast,
) : GraphQLViewModel() {
  val state = SpaceSettingsScreenState(viewModelScope)

  val query = watchQuery(
    placeholderData = placeholderData(),
    onInitialData = { data ->
      state.form.name.initialValue = data.site.name
      state.form.slug.initialValue = data.site.slug
      state.form.logoId.initialValue = data.site.logo.id
    },
  ) { SpaceSettingsScreen_Query(siteId = siteService.siteId) }

  val usersiteHost: String = Konfig.USERSITE_HOST
    .trim()
    .removePrefix("*.")
    .removePrefix(".")

  suspend fun uploadLogo(file: PlatformFile) {
    try {
      toast.withLoading(
        message = "로고 업로드 중...",
        errorMessage = "로고 업로드에 실패했어요. 다시 시도해주세요.",
      ) {
        val path = blobService.uploadBytes(
          bytes = file.bytes,
          filename = file.filename,
          mimeType = file.mimeType,
        )

        val result = executeMutation(
          SpaceSettingsScreen_PersistBlobAsImage_Mutation(
            input = PersistBlobAsImageInput(path = path),
          ),
        )

        state.logoPreviewUrl = result.persistBlobAsImage.img_image.url
        state.form.logoId.value = result.persistBlobAsImage.id
        success("로고가 업로드되었어요.")
      }
    } catch (e: Exception) {
      Logger.e(e) { "Failed to upload logo" }
    }
  }

  fun submit(onSubmit: suspend () -> Unit) {
    viewModelScope.launch {
      state.isSubmitting = true
      try {
        if (!state.form.validate()) return@launch

        executeMutation(
          SpaceSettingsScreen_UpdateSite_Mutation(
            input = UpdateSiteInput(
              siteId = siteService.siteId,
              name = Optional.present(state.form.name.value.trim()),
              logoId = Optional.present(state.form.logoId.value),
            ),
          ),
        )

        executeMutation(
          SpaceSettingsScreen_UpdateSiteSlug_Mutation(
            input = UpdateSiteSlugInput(
              siteId = siteService.siteId,
              slug = state.form.slug.value.trim().lowercase(),
            ),
          ),
        )

        toast.show(ToastType.Success, "스페이스 설정이 변경되었어요.")
        state.form.commit()

        onSubmit()
      } catch (e: TypieError) {
        if (e.code == "site_slug_already_exists") {
          state.form.slug.errors = listOf("이미 존재하는 스페이스 주소예요.")
        } else {
          toast.show(ToastType.Error, e.message ?: "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to update site settings" }
        toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      } finally {
        state.isSubmitting = false
      }
    }
  }

  fun deleteSite(onDeleted: suspend () -> Unit) {
    viewModelScope.launch {
      state.isDeleting = true
      try {
        executeMutation(
          SpaceSettingsScreen_DeleteSite_Mutation(
            input = DeleteSiteInput(siteId = siteService.siteId),
          ),
        )

        apolloClient.apolloStore.remove(CacheKey(query.data.me.id))

        val remainingSiteIds = query.data.me.sites.map { it.id }.filter { it != siteService.siteId }
        siteService.siteId = remainingSiteIds.first()

        toast.show(ToastType.Success, "스페이스가 삭제되었어요.")

        onDeleted()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to delete site" }
        toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      } finally {
        state.isDeleting = false
      }
    }
  }
}

private fun placeholderData() = SpaceSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
  }
  site = buildSite {
    name = text(3..8)
    slug = text(4..10)
  }
}
