package co.typie.editor.input

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
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
  fun `deletion and empty composition match the native buffer and Rust fixtures`() {
    val fixture =
      File("../../../crates/editor-core/src/tests/fixtures/ime-delete-composition.json").readText()
    val messages = Json.decodeFromString<Map<String, List<Message>>>(fixture)
    for (name in messages.keys) {
      val ime =
        Ime(
          text = "\u2028abcd\u2029",
          windowStart = 0,
          selection = if (name == "delete-before-selection") ImeRange(2, 4) else ImeRange(3, 3),
          composing =
            when (name) {
              "delete-inside-composition" -> ImeRange(2, 4)
              "empty-composition" -> ImeRange(2, 3)
              else -> null
            },
        )
      val commands =
        when (name) {
          "delete-before-selection" ->
            listOf(DeleteSurroundingTextCommand(1, 0), CommitTextCommand("X", 1))
          "delete-inside-composition" ->
            listOf(
              DeleteSurroundingTextCommand(1, 0),
              SetSelectionCommand(3, 3),
              CommitTextCommand("X", 1),
            )
          "empty-composition" ->
            listOf(
              SetComposingTextCommand("", 1),
              SetSelectionCommand(3, 3),
              CommitTextCommand("X", 1),
            )
          else -> error("Unknown fixture: $name")
        }
      val expected =
        when (name) {
          "delete-before-selection" -> "Xd" to 2
          "delete-inside-composition" -> "aXd" to 3
          else -> "acXd" to 4
        }
      val native = ime.toEditProcessor().apply(commands)
      assertEquals("\u2028${expected.first}\u2029", native.text, name)
      assertEquals(TextRange(expected.second), native.selection, name)
      assertEquals(null, native.composition, name)
      assertEquals(
        messages.getValue(name),
        EditorImeCommandNormalizer.normalize(commands, ime),
        name,
      )
    }
  }

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
