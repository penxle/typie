package co.typie.screen.settings.ai_settings

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
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonPrimitive

class AiSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      AiSettingsScreen_Query()
    }

  var aiOptIn by mutableStateOf(false)
    private set

  var isUpdatingAiOptIn by mutableStateOf(false)
    private set

  private var hasInitializedAiOptIn by mutableStateOf(false)

  fun initializeAiOptIn(enabled: Boolean) {
    if (hasInitializedAiOptIn) return
    aiOptIn = enabled
    hasInitializedAiOptIn = true
  }

  suspend fun updateAiOptIn(enabled: Boolean): Result<Unit, Nothing> {
    if (isUpdatingAiOptIn || aiOptIn == enabled) return Result.Ok(Unit)

    val previous = aiOptIn
    aiOptIn = enabled
    return loading<Unit, Nothing>({ isUpdatingAiOptIn = it }) {
        Apollo.executeMutation(
          AiSettingsScreen_UpdatePreferences_Mutation(
            input =
              UpdatePreferencesInput(value = JsonObject(mapOf("aiOptIn" to JsonPrimitive(enabled))))
          )
        )
        query.refetch()
      }
      .also { result ->
        if (result is Result.Exception) {
          aiOptIn = previous
        }
      }
  }
}

internal fun JsonElement.aiOptIn(): Boolean {
  val json = this as? JsonObject ?: return false
  return json["aiOptIn"]?.jsonPrimitive?.booleanOrNull ?: false
}

private fun placeholderData() =
  AiSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { preferences = JsonObject(emptyMap()) }
  }
