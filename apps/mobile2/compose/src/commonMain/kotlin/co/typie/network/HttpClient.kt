package co.typie.graphql

import co.typie.dev.NetworkPreset
import co.typie.dev.NetworkSimulator
import co.typie.dev.SimulatedNetworkFailureException
import io.ktor.client.HttpClient
import io.ktor.client.plugins.HttpCallValidator
import io.ktor.client.plugins.HttpSend
import io.ktor.client.plugins.ResponseException
import io.ktor.client.plugins.plugin
import io.ktor.client.plugins.websocket.WebSockets
import kotlinx.coroutines.delay

val Http: HttpClient =
  HttpClient {
      followRedirects = false

      install(WebSockets)

      install(HttpCallValidator) {
        validateResponse { response ->
          if (response.status.value > 399) {
            throw ResponseException(response, "Error: ${response.status}")
          }
        }
      }
    }
    .apply {
      plugin(HttpSend).intercept { context ->
        when (NetworkSimulator.preset.value) {
          NetworkPreset.Normal -> execute(context)
          NetworkPreset.Slow -> {
            delay(2000L)
            execute(context)
          }
          NetworkPreset.Offline -> throw SimulatedNetworkFailureException()
        }
      }
    }
