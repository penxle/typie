package co.typie.screen.preset_settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.graphql.PresetSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import kotlinx.serialization.json.JsonObject
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
internal class PresetSettingsViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  internal val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { PresetSettingsScreen_Query() }

  internal val currentTemplate: PresetTemplate
    get() = PresetTemplate.fromPreferencesJson(query.data.me.preferences)

  internal val activeDocumentFontFamilies: List<PresetFontFamily>
    get() = activePresetFontFamiliesFromPresetQuery(query.data.me.documentFontFamilies)

  internal val normalizedFontFamilyOptions: List<PresetOption<String>>
    get() = normalizedPresetFontFamilyOptions(activeDocumentFontFamilies)

  internal val selectedFontWeightAvailability: List<PresetOption<Int>>
    get() = selectedFontWeightAvailabilityOptions(currentTemplate, activeDocumentFontFamilies)

  internal suspend fun saveTemplate(nextTemplate: PresetTemplate): Result<Unit, Nothing> {
    return updateTemplate(
      value = JsonObject(mapOf("template" to nextTemplate.toJsonObject())),
    )
  }

  internal suspend fun resetTemplate(): Result<Unit, Nothing> {
    return updateTemplate(
      value = JsonObject(mapOf("template" to JsonObject(emptyMap()))),
    )
  }

  private suspend fun updateTemplate(value: JsonObject): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      PresetSettingsScreen_UpdatePreferences_Mutation(
        input = UpdatePreferencesInput(value = value),
      ),
    )
    query.refetch()
  }
}

private fun placeholderData() = PresetSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    preferences = JsonObject(emptyMap())
    documentFontFamilies = emptyList()
  }
}
