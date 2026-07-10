package co.typie.screen.settings.presetsettings

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.FontLoader
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.graphql.PresetSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.QueryState
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.serialization.json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement

internal class PresetSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      PresetSettingsScreen_Query()
    }

  init {
    FontLoader.watchFonts(viewModelScope) {
      (query.state as? QueryState.Success)
        ?.data
        ?.me
        ?.documentFontFamilies
        ?.filter { it.editorSettingsFontFamily_family.state == FontFamilyState.ACTIVE }
        ?.map { family ->
          val activeFontIds =
            family.editorSettingsFontFamily_family.fonts
              .filter { it.state == FontState.ACTIVE }
              .map { it.id }
              .toSet()
          family.fontLoader_FontFamily.copy(
            fonts = family.fontLoader_FontFamily.fonts.filter { it.id in activeFontIds }
          )
        }
    }
  }

  val preset by derivedStateOf {
    json.decodeFromJsonElement<PresetPreferences>(query.data.me.preferences).template ?: Preset()
  }

  val fontFamilies by derivedStateOf {
    query.data.me.documentFontFamilies.map { family -> family.editorSettingsFontFamily_family }
  }

  internal suspend fun updatePreset(preset: Preset): Result<Unit, Nothing> = result {
    val value = json.encodeToJsonElement(PresetPreferences(template = preset))
    Apollo.executeMutation(
      PresetSettingsScreen_UpdatePreferences_Mutation(
        input = UpdatePreferencesInput(value = value)
      ),
      optimisticUpdate =
        PresetSettingsScreen_UpdatePreferences_Mutation.Data {
          updatePreferences = buildUser {
            id = query.data.me.id
            preferences = value
          }
        },
    )
  }

  internal suspend fun resetPreset(): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      PresetSettingsScreen_UpdatePreferences_Mutation(
        input =
          UpdatePreferencesInput(
            value = json.encodeToJsonElement(PresetPreferences(template = null))
          )
      )
    )
  }
}

private fun placeholderData() =
  PresetSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      preferences = JsonObject(emptyMap())
      documentFontFamilies = emptyList()
    }
  }
