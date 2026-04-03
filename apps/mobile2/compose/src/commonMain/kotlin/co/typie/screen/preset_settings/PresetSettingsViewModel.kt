package co.typie.screen.preset_settings

import co.touchlab.kermit.Logger
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.graphql.PresetSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.JsonObject
import org.koin.core.annotation.KoinViewModel

private const val MUTATION_FAILURE_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

@KoinViewModel
internal class PresetSettingsViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  internal val query = watchQuery(placeholderData()) { PresetSettingsScreen_Query() }

  internal val currentTemplate: PresetTemplate
    get() = PresetTemplate.fromPreferencesJson(query.data.me.preferences)

  internal val activeDocumentFontFamilies: List<PresetFontFamily>
    get() = activePresetFontFamiliesFromPresetQuery(query.data.me.documentFontFamilies)

  internal val normalizedFontFamilyOptions: List<PresetOption<String>>
    get() = normalizedPresetFontFamilyOptions(activeDocumentFontFamilies)

  internal val selectedFontWeightAvailability: List<PresetOption<Int>>
    get() = selectedFontWeightAvailabilityOptions(currentTemplate, activeDocumentFontFamilies)

  internal suspend fun saveTemplate(nextTemplate: PresetTemplate): Boolean {
    return updateTemplate(
      value = JsonObject(mapOf("template" to nextTemplate.toJsonObject())),
    )
  }

  internal suspend fun resetTemplate(): Boolean {
    return updateTemplate(
      value = JsonObject(mapOf("template" to JsonObject(emptyMap()))),
    )
  }

  private suspend fun updateTemplate(value: JsonObject): Boolean {
    try {
      executeMutation(
        PresetSettingsScreen_UpdatePreferences_Mutation(
          input = UpdatePreferencesInput(value = value),
        ),
      )
      query.refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to update preset template" }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
      return false
    }
  }
}

private fun placeholderData() = PresetSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    preferences = JsonObject(emptyMap())
    documentFontFamilies = emptyList()
  }
}
