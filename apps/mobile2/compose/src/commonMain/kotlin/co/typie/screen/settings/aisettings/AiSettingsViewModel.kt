package co.typie.screen.settings.aisettings

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.AiSettingsScreen_Query
import co.typie.graphql.AiSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import co.typie.serialization.json
import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement

@Serializable data class AiPreferences(@EncodeDefault val aiOptIn: Boolean = false)

class AiSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      AiSettingsScreen_Query()
    }

  var isSubmitting by mutableStateOf(false)
    private set

  val aiOptIn by derivedStateOf {
    json.decodeFromJsonElement<AiPreferences>(query.data.me.preferences).aiOptIn
  }

  suspend fun updateAiOptIn(enabled: Boolean): Result<Unit, Nothing> {
    if (isSubmitting || aiOptIn == enabled) return Result.Ok(Unit)

    return loading({ isSubmitting = it }) {
      Apollo.executeMutation(
        AiSettingsScreen_UpdatePreferences_Mutation(
          input =
            UpdatePreferencesInput(
              value = json.encodeToJsonElement(AiPreferences(aiOptIn = enabled))
            )
        )
      )
    }
  }
}

private fun placeholderData() =
  AiSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { preferences = JsonObject(emptyMap()) }
  }
