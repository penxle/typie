package co.typie.domain.onboarding

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.Apollo
import co.typie.graphql.OnboardingGate_Query
import co.typie.graphql.OnboardingGate_UpdatePreferences_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.serialization.json
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import kotlin.time.Clock
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.encodeToJsonElement

enum class OnboardingGateState {
  Unknown,
  Show,
  Hide,
}

object OnboardingService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

  var state by mutableStateOf(OnboardingGateState.Unknown)
    private set

  private var generation = 0

  fun reset() {
    generation++
    state = OnboardingGateState.Unknown
  }

  suspend fun evaluate() {
    if (state != OnboardingGateState.Unknown) return
    val startedGeneration = generation
    val result =
      try {
        withTimeout(5.seconds) {
          val response =
            Apollo.query(OnboardingGate_Query()).fetchPolicy(FetchPolicy.NetworkOnly).execute()
          val me = response.dataOrThrow().me
          val preferences = me.preferences as? JsonObject ?: JsonObject(emptyMap())
          if (shouldShowOnboarding(me.createdAt, preferences, Clock.System.now())) {
            OnboardingGateState.Show
          } else {
            OnboardingGateState.Hide
          }
        }
      } catch (e: TimeoutCancellationException) {
        OnboardingGateState.Hide
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        OnboardingGateState.Hide
      }
    if (generation == startedGeneration) state = result
  }

  fun complete() {
    state = OnboardingGateState.Hide
    scope.launch {
      try {
        Apollo.executeMutation(
          OnboardingGate_UpdatePreferences_Mutation(
            input =
              UpdatePreferencesInput(
                value =
                  json.encodeToJsonElement(
                    mapOf(ONBOARDING_COMPLETED_KEY to Clock.System.now().toString())
                  )
              )
          )
        )
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {}
    }
  }
}
