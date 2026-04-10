package co.typie.screen.settings.preset_settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.graphql.PresetSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import kotlinx.serialization.json.JsonObject

internal class PresetSettingsViewModel : ViewModel() {
  internal val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      PresetSettingsScreen_Query()
    }

  internal val currentTemplate: PresetTemplate
    get() = PresetTemplate.fromPreferencesJson(query.data.me.preferences)

  internal val activeDocumentFontFamilies: List<PresetFontFamily>
    get() = activePresetFontFamiliesFromPresetQuery(query.data.me.documentFontFamilies)

  internal val normalizedFontFamilyOptions: List<PresetOption<String>>
    get() = normalizedPresetFontFamilyOptions(activeDocumentFontFamilies)

  internal val selectedFontWeightAvailability: List<PresetOption<Int>>
    get() = selectedFontWeightAvailabilityOptions(currentTemplate, activeDocumentFontFamilies)

  internal suspend fun saveTemplate(nextTemplate: PresetTemplate): Result<Unit, Nothing> {
    return updateTemplate(value = JsonObject(mapOf("template" to nextTemplate.toJsonObject())))
  }

  internal suspend fun resetTemplate(): Result<Unit, Nothing> {
    return updateTemplate(value = JsonObject(mapOf("template" to JsonObject(emptyMap()))))
  }

  private suspend fun updateTemplate(value: JsonObject): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      PresetSettingsScreen_UpdatePreferences_Mutation(input = UpdatePreferencesInput(value = value))
    )
    query.refetch()
  }
}

private fun placeholderData() =
  PresetSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      preferences = JsonObject(emptyMap())
      documentFontFamilies = emptyList()
    }
  }
