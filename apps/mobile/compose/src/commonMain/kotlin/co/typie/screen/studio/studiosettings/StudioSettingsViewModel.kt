package co.typie.screen.studio.studiosettings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.Konfig
import co.typie.domain.blob.BlobService
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.maxLength
import co.typie.form.minLength
import co.typie.form.pattern
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.StudioSettingsScreen_DeleteSite_Mutation
import co.typie.graphql.StudioSettingsScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.StudioSettingsScreen_Query
import co.typie.graphql.StudioSettingsScreen_UpdateSiteSlug_Mutation
import co.typie.graphql.StudioSettingsScreen_UpdateSite_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildSite
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.DeleteSiteInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.graphql.type.SiteDateDisplay
import co.typie.graphql.type.UpdateSiteInput
import co.typie.graphql.type.UpdateSiteSlugInput
import co.typie.graphql.watchQuery
import co.typie.platform.PickedFile
import co.typie.result.Result
import co.typie.result.Task
import co.typie.result.loading
import co.typie.result.task
import co.typie.storage.Preference
import com.apollographql.apollo.api.Optional
import com.apollographql.cache.normalized.api.CacheKey
import com.apollographql.cache.normalized.apolloStore
import kotlinx.coroutines.CoroutineScope

private val UNAVAILABLE_SITE_SLUGS =
  listOf("admin", "app", "cname", "dev", "docs", "help", "template", "www")

class StudioSettingsForm(scope: CoroutineScope) : FormState(scope, autoFocusFirstField = false) {
  val name =
    field("") {
      required("작업실 이름을 입력해주세요.")
      validateOn(ValidateOn.Change) { minLength(1, "작업실 이름을 입력해주세요.") }
    }

  val slug =
    field("") {
      required("주소를 입력해주세요.")
      validateOn(ValidateOn.Change) {
        minLength(4, "주소는 4글자 이상이여야 해요")
        maxLength(63, "주소는 63글자를 넘을 수 없어요")
        pattern(Regex("^[\\da-z-]+$"), "주소는 소문자, 숫자, 하이픈만 사용할 수 있어요")
        pattern(Regex("^(?!.*--)[\\da-z-]+$"), "하이픈을 연속으로 사용할 수 없어요")
        pattern(Regex("^[\\da-z][\\da-z-]*[\\da-z]$"), "주소는 하이픈으로 시작하거나 끝날 수 없어요")
        rule { if (it in UNAVAILABLE_SITE_SLUGS) "사용할 수 없는 주소예요" else null }
      }
    }

  val logoId = field("") { focusable = false }

  val dateDisplay = field(SiteDateDisplay.UPDATED_AT) { focusable = false }
}

sealed interface StudioSettingsError {
  data object ValidationFailed : StudioSettingsError

  data object SlugAlreadyExists : StudioSettingsError
}

class StudioSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { Preference.siteId == null },
      onInitialData = { data ->
        form.name.initialValue = data.site.name
        form.slug.initialValue = data.site.slug
        form.logoId.initialValue = data.site.logo.id
        form.dateDisplay.initialValue = data.site.dateDisplay
      },
    ) {
      StudioSettingsScreen_Query(siteId = Preference.siteId!!)
    }

  val form = StudioSettingsForm(viewModelScope)
  var logoPreviewUrl: String? by mutableStateOf(null)

  var isSubmitting by mutableStateOf(false)
    private set

  var isDeleting by mutableStateOf(false)
    private set

  val usersiteHost: String = Konfig.USERSITE_HOST.removePrefix("*.").removePrefix(".")

  fun uploadLogo(file: PickedFile): Task<Unit, Unit, Nothing> = task {
    emit(Unit)

    val path = BlobService.upload(file)

    val image =
      Apollo.executeMutation(
        StudioSettingsScreen_PersistBlobAsImage_Mutation(
          input = PersistBlobAsImageInput(path = path)
        )
      )

    logoPreviewUrl = image.persistBlobAsImage.img_image.url
    form.logoId.value = image.persistBlobAsImage.id
  }

  suspend fun submit(): Result<Unit, StudioSettingsError> {
    if (!form.validate()) return Result.Err(StudioSettingsError.ValidationFailed)

    return loading({ isSubmitting = it }) {
      Apollo.executeMutation(
        StudioSettingsScreen_UpdateSite_Mutation(
          input =
            UpdateSiteInput(
              siteId = Preference.siteId!!,
              name = Optional.present(form.name.value),
              logoId = Optional.present(form.logoId.value),
              dateDisplay = Optional.present(form.dateDisplay.value),
            )
        )
      )

      if (form.slug.isDirty) {
        try {
          Apollo.executeMutation(
            StudioSettingsScreen_UpdateSiteSlug_Mutation(
              input = UpdateSiteSlugInput(siteId = Preference.siteId!!, slug = form.slug.value)
            )
          )
        } catch (e: TypieError) {
          if (e.code == "site_slug_already_exists") {
            form.slug.errors = listOf("이미 사용 중인 URL이에요.")
            form.focusFirstError()
            raise(StudioSettingsError.ValidationFailed)
          }

          throw e
        }
      }

      form.commit()
    }
  }

  // TODO: 작업실 삭제 트래킹
  suspend fun deleteSite(): Result<Unit, Nothing> =
    loading({ isDeleting = it }) {
      Apollo.executeMutation(
        StudioSettingsScreen_DeleteSite_Mutation(
          input = DeleteSiteInput(siteId = Preference.siteId!!)
        )
      )

      Apollo.apolloStore.remove(CacheKey(query.data.me.id))

      val remainingSiteIds = query.data.me.sites.map { it.id }.filter { it != Preference.siteId!! }
      Preference.siteId = remainingSiteIds.first()
    }
}

private fun placeholderData() =
  StudioSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { name = text(3..6) }
    site = buildSite {
      name = text(3..8)
      slug = text(4..10)
      dateDisplay = SiteDateDisplay.UPDATED_AT
    }
  }
