@file:OptIn(ExperimentalTime::class)

package co.typie.editor.input

import co.typie.network.Http
import co.typie.platform.PlatformModule
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlinx.serialization.Serializable

private const val InputLogEndpoint = "https://ime.penxle.io"

@Serializable
internal data class InputLogPayload(
  val schema: String,
  val name: String,
  val timestamp: String,
  val platform: String,
  val device: InputLogDevice,
  val app: InputLogApp,
  val entries: List<RecordedInputEntry>,
)

@Serializable
internal data class InputLogDevice(val model: String, val os: String, val keyboard: String?)

@Serializable internal data class InputLogApp(val version: String, val build: String)

internal fun buildInputLogPayload(
  name: String,
  entries: List<RecordedInputEntry>,
): InputLogPayload {
  val info = PlatformModule.deviceInfo.retrieve()
  return InputLogPayload(
    schema = "app2/v1",
    name = name,
    timestamp = Clock.System.now().toString(),
    platform = PlatformModule.platform.name.lowercase(),
    device =
      InputLogDevice(
        model = info.model,
        os = "${info.osName} ${info.osVersion}",
        keyboard = currentKeyboardId(),
      ),
    app = InputLogApp(version = info.appVersion, build = info.appBuildNumber),
    entries = entries,
  )
}

internal suspend fun sendInputLog(payload: InputLogPayload) {
  Http.post(InputLogEndpoint) {
    contentType(ContentType.Application.Json)
    setBody(payload)
  }
}
