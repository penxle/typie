package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.json.Json

class EditorImeCoordinateContractTest {
  @Test
  fun `multiline selection messages match the Rust integration fixture`() {
    val ime =
      Ime(text = "\u2028\u2029", windowStart = 0, selection = ImeRange(1, 1), composing = null)
    val messages =
      EditorImeCommandNormalizer.normalize(
        listOf(CommitTextCommand("a\nb", 1), SetSelectionCommand(4, 4), CommitTextCommand("X", 1)),
        ime,
      )
    val fixture =
      File("../../../crates/editor-core/src/tests/fixtures/ime-multiline-selection.json").readText()
    assertEquals(Json.decodeFromString<List<Message>>(fixture), messages)
  }
}
