package co.typie.editor.input

import co.typie.serialization.json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.encodeToString

class InputLogPayloadSerializationTest {
  @Test
  fun `payload serializes with app2 v1 schema marker`() {
    val payload =
      InputLogPayload(
        schema = "app2/v1",
        name = "테스트",
        timestamp = "2026-07-11T00:00:00Z",
        platform = "android",
        device =
          InputLogDevice(
            model = "Pixel 8",
            os = "Android 16",
            keyboard = "com.google.android.inputmethod.latin/...",
          ),
        app = InputLogApp(version = "1.0.0", build = "100"),
        entries = listOf(RecordedInputEntry.Session(seq = 1, t = 0, event = "start")),
      )
    assertEquals(
      """{"schema":"app2/v1","name":"테스트","timestamp":"2026-07-11T00:00:00Z","platform":"android",""" +
        """"device":{"model":"Pixel 8","os":"Android 16","keyboard":"com.google.android.inputmethod.latin/..."},""" +
        """"app":{"version":"1.0.0","build":"100"},""" +
        """"entries":[{"type":"session","seq":1,"t":0,"event":"start"}]}""",
      json.encodeToString(payload),
    )
  }
}
