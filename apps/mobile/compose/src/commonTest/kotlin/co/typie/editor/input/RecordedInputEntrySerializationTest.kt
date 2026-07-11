package co.typie.editor.input

import androidx.compose.ui.text.input.CommitTextCommand
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.serialization.json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.encodeToString

class RecordedInputEntrySerializationTest {
  @Test
  fun `imeCall serializes flat with type discriminator`() {
    val entry: RecordedInputEntry =
      RecordedInputEntry.ImeCall(
        seq = 1,
        t = 10,
        method = "commitText",
        args = "text=가, newCursorPosition=1",
      )
    assertEquals(
      """{"type":"imeCall","seq":1,"t":10,"method":"commitText","args":"text=가, newCursorPosition=1"}""",
      json.encodeToString(entry),
    )
  }

  @Test
  fun `imeRead serializes null result explicitly`() {
    val entry: RecordedInputEntry =
      RecordedInputEntry.ImeRead(
        seq = 2,
        t = 20,
        method = "getSelectedText",
        args = "flags=0",
        result = null,
      )
    assertEquals(
      """{"type":"imeRead","seq":2,"t":20,"method":"getSelectedText","args":"flags=0","result":null}""",
      json.encodeToString(entry),
    )
  }

  @Test
  fun `dispatch serializes ffi messages and ime snapshots`() {
    val entry: RecordedInputEntry =
      RecordedInputEntry.Dispatch(
        seq = 3,
        t = 30,
        messages = listOf(Message.TextInput(listOf(FlatImeOp.Compose("ㅎ")))),
        imeBefore = Ime(text = "", windowStart = 0, selection = ImeRange(0, 0), composing = null),
        imeAfter =
          Ime(text = "ㅎ", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1)),
      )
    assertEquals(
      """{"type":"dispatch","seq":3,"t":30,"messages":[{"type":"text_input","ops":[{"type":"compose","text":"ㅎ"}]}],""" +
        """"imeBefore":{"text":"","window_start":0,"selection":{"start":0,"end":0},"composing":null},""" +
        """"imeAfter":{"text":"ㅎ","window_start":0,"selection":{"start":1,"end":1},"composing":{"start":0,"end":1}}}""",
      json.encodeToString(entry),
    )
  }

  @Test
  fun `editCommands serializes decision tag`() {
    val entry: RecordedInputEntry =
      RecordedInputEntry.EditCommands(
        seq = 4,
        t = 40,
        commands = listOf("CommitText(text=a, newCursorPosition=1)"),
        decision = RecordedBridgeDecision.Normalize,
        messages = listOf(Message.TextInput(listOf(FlatImeOp.ReplaceSelection("a")))),
        imeBefore = null,
        imeAfter = null,
      )
    assertEquals(
      """{"type":"editCommands","seq":4,"t":40,"commands":["CommitText(text=a, newCursorPosition=1)"],""" +
        """"decision":"normalize","messages":[{"type":"text_input","ops":[{"type":"replace_selection","text":"a"}]}],""" +
        """"imeBefore":null,"imeAfter":null}""",
      json.encodeToString(entry),
    )
  }

  @Test
  fun `null intercept classifies as normalize`() {
    assertEquals(RecordedBridgeDecision.Normalize, classifyBridgeRoute(null))
  }

  @Test
  fun `empty intercept classifies as drop`() {
    assertEquals(RecordedBridgeDecision.Drop, classifyBridgeRoute(emptyList()))
  }

  @Test
  fun `non-empty intercept classifies as replay`() {
    assertEquals(
      RecordedBridgeDecision.Replay,
      classifyBridgeRoute(
        listOf(Message.Key(co.typie.editor.ffi.KeyEvent(co.typie.editor.ffi.Key.Enter)))
      ),
    )
  }

  @Test
  fun `editCommands entry carries normalizer output verbatim`() {
    val commands = listOf(CommitTextCommand("a", 1))
    val messages = EditorImeCommandNormalizer.normalize(commands = commands, ime = null)
    val entry =
      RecordedInputEntry.EditCommands(
        seq = 5,
        t = 50,
        commands = commands.map { it.describe() },
        decision = classifyBridgeRoute(null),
        messages = messages,
        imeBefore = null,
        imeAfter = null,
      )
    assertEquals(
      listOf(Message.TextInput(listOf(FlatImeOp.Compose("a"), FlatImeOp.CommitAsIs))),
      entry.messages,
    )
    assertEquals(listOf("CommitText(text=a, newCursorPosition=1)"), entry.commands)
  }
}
