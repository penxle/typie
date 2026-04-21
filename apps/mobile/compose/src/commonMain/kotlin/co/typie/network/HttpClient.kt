package co.typie.network

import co.typie.dev.NetworkPreset
import co.typie.dev.NetworkSimulator
import co.typie.dev.SimulatedNetworkFailureException
import co.typie.serialization.json
import io.ktor.client.HttpClient
import io.ktor.client.plugins.HttpSend
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.plugin
import io.ktor.client.plugins.websocket.WebSockets
import io.ktor.serialization.kotlinx.json.json
import kotlinx.coroutines.delay

val Http: HttpClient =
  HttpClient {
      expectSuccess = true
      followRedirects = false

      install(WebSockets)
      install(ContentNegotiation) { json(json) }
    }
    .apply {
      plugin(HttpSend).intercept { context ->
        when (NetworkSimulator.preset) {
          NetworkPreset.Normal -> execute(context)
          NetworkPreset.Slow -> {
            delay(2000L)
            execute(context)
          }
          NetworkPreset.Offline -> throw SimulatedNetworkFailureException()
        }
      }
    }
