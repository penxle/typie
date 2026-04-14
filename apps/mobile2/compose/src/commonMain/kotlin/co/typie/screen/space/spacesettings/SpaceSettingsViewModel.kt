package co.typie.screen.space.spacesettings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.Konfig
import co.typie.domain.blob.BlobService
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.maxLength
import co.typie.form.minLength
import co.typie.form.pattern
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SpaceSettingsScreen_DeleteSite_Mutation
import co.typie.graphql.SpaceSettingsScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.SpaceSettingsScreen_Query
import co.typie.graphql.SpaceSettingsScreen_UpdateSiteSlug_Mutation
import co.typie.graphql.SpaceSettingsScreen_UpdateSite_Mutation
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
import co.typie.platform.PlatformFile
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

class SpaceSettingsForm(scope: CoroutineScope) : FormState(scope) {
  val name =
    field("") {
      required("스페이스 이름을 입력해주세요.")
      validateOn(ValidateOn.Change) { minLength(1, "스페이스 이름을 입력해주세요.") }
    }

  val slug =
    field("") {
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

  val logoId = field("") { focusable = false }

  val dateDisplay = field(SiteDateDisplay.UPDATED_AT) { focusable = false }
}

class SpaceSettingsScreenState(scope: CoroutineScope) {
  val form = SpaceSettingsForm(scope)
  var logoPreviewUrl: String? by mutableStateOf(null)
}

sealed interface SubmitError {
  data object SlugAlreadyExists : SubmitError

  data object SubscriptionUnknown : SubmitError
}

class SpaceSettingsViewModel : ViewModel() {
  private val blobService = BlobService
  private val subscriptionService = SubscriptionService
  val state = SpaceSettingsScreenState(viewModelScope)
  var isSubmitting by mutableStateOf(false)
    private set

  var isDeletingSite by mutableStateOf(false)
    private set

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { Preference.siteId == null },
      onInitialData = { data ->
        state.form.name.initialValue = data.site.name
        state.form.slug.initialValue = data.site.slug
        state.form.logoId.initialValue = data.site.logo.id
        state.form.dateDisplay.initialValue = data.site.dateDisplay
      },
    ) {
      SpaceSettingsScreen_Query(siteId = Preference.siteId!!)
    }

  val usersiteHost: String = Konfig.USERSITE_HOST.trim().removePrefix("*.").removePrefix(".")

  // TODO: 로고 변경 트래킹
  fun uploadLogo(file: PlatformFile): Task<Unit, Unit, Nothing> = task {
    emit(Unit)

    val path =
      blobService.uploadBytes(
        bytes = file.bytes,
        filename = file.filename,
        mimeType = file.mimeType,
      )

    val image =
      Apollo.executeMutation(
        SpaceSettingsScreen_PersistBlobAsImage_Mutation(
          input = PersistBlobAsImageInput(path = path)
        )
      )

    state.logoPreviewUrl = image.persistBlobAsImage.img_image.url
    state.form.logoId.value = image.persistBlobAsImage.id
  }

  suspend fun submit(): Result<Unit, SubmitError> {
    if (!state.form.validate()) return Result.Ok(Unit)

    return loading({ isSubmitting = it }) {
      Apollo.executeMutation(
        SpaceSettingsScreen_UpdateSite_Mutation(
          input =
            UpdateSiteInput(
              siteId = Preference.siteId!!,
              name = Optional.present(state.form.name.value.trim()),
              logoId = Optional.present(state.form.logoId.value),
              dateDisplay = Optional.present(state.form.dateDisplay.value),
            )
        )
      )

      if (state.form.slug.isDirty) {
        when (subscriptionService.state) {
          is SubscriptionServiceState.Subscribed -> {
            try {
              Apollo.executeMutation(
                SpaceSettingsScreen_UpdateSiteSlug_Mutation(
                  input =
                    UpdateSiteSlugInput(
                      siteId = Preference.siteId!!,
                      slug = state.form.slug.value.trim().lowercase(),
                    )
                )
              )
            } catch (e: TypieError) {
              if (e.code == "site_slug_already_exists") raise(SubmitError.SlugAlreadyExists)
              throw e
            }
          }

          is SubscriptionServiceState.NotSubscribed -> Unit

          is SubscriptionServiceState.Unknown -> raise(SubmitError.SubscriptionUnknown)
        }
      }

      state.form.commit()
      query.refetch()
    }
  }

  // TODO: 스페이스 삭제 트래킹
  suspend fun deleteSite(): Result<Unit, Nothing> =
    loading({ isDeletingSite = it }) {
      Apollo.executeMutation(
        SpaceSettingsScreen_DeleteSite_Mutation(
          input = DeleteSiteInput(siteId = Preference.siteId!!)
        )
      )

      Apollo.apolloStore.remove(CacheKey(query.data.me.id))

      val remainingSiteIds = query.data.me.sites.map { it.id }.filter { it != Preference.siteId!! }
      Preference.siteId = remainingSiteIds.first()
    }
}

private fun placeholderData() =
  SpaceSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { name = text(3..6) }
    site = buildSite {
      name = text(3..8)
      slug = text(4..10)
      dateDisplay = SiteDateDisplay.UPDATED_AT
    }
  }
