package co.typie.editor.input

import androidx.compose.ui.text.input.BackspaceCommand
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextCommand
import androidx.compose.ui.text.input.DeleteSurroundingTextInCodePointsCommand
import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.FinishComposingTextCommand
import androidx.compose.ui.text.input.MoveCursorCommand
import androidx.compose.ui.text.input.SetComposingRegionCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.Message
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
internal sealed interface RecordedInputEntry {
  val seq: Long
  val t: Long

  @Serializable
  @SerialName("imeCall")
  data class ImeCall(
    override val seq: Long,
    override val t: Long,
    val method: String,
    val args: String,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("imeRead")
  data class ImeRead(
    override val seq: Long,
    override val t: Long,
    val method: String,
    val args: String,
    val result: String?,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("updateSelection")
  data class UpdateSelection(
    override val seq: Long,
    override val t: Long,
    val selStart: Int,
    val selEnd: Int,
    val composingStart: Int,
    val composingEnd: Int,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("sessionStart")
  data class SessionStart(
    override val seq: Long,
    override val t: Long,
    val initialSelStart: Int,
    val initialSelEnd: Int,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("dispatch")
  data class Dispatch(
    override val seq: Long,
    override val t: Long,
    val messages: List<Message>,
    val imeBefore: Ime?,
    val imeAfter: Ime?,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("editCommands")
  data class EditCommands(
    override val seq: Long,
    override val t: Long,
    val commands: List<String>,
    val decision: RecordedBridgeDecision,
    val messages: List<Message>,
    val imeBefore: Ime?,
    val imeAfter: Ime?,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("valuePull")
  data class ValuePull(
    override val seq: Long,
    override val t: Long,
    val text: String,
    val selectionStart: Int,
    val selectionEnd: Int,
    val compositionStart: Int?,
    val compositionEnd: Int?,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("keyEvent")
  data class HardwareKey(
    override val seq: Long,
    override val t: Long,
    val key: String,
    val stage: String,
    val matchedBinding: Boolean,
    val blockedByComposition: Boolean,
    val consumed: Boolean,
    val text: String? = null,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("toolbarInput")
  data class ToolbarInput(
    override val seq: Long,
    override val t: Long,
    val method: String,
    val args: String?,
  ) : RecordedInputEntry

  @Serializable
  @SerialName("session")
  data class Session(
    override val seq: Long,
    override val t: Long,
    val event: String,
  ) : RecordedInputEntry
}

@Serializable
internal enum class RecordedBridgeDecision {
  @SerialName("drop") Drop,
  @SerialName("replay") Replay,
  @SerialName("normalize") Normalize,
}

internal fun classifyBridgeRoute(intercepted: List<Message>?): RecordedBridgeDecision =
  when {
    intercepted == null -> RecordedBridgeDecision.Normalize
    intercepted.isEmpty() -> RecordedBridgeDecision.Drop
    else -> RecordedBridgeDecision.Replay
  }

internal fun EditCommand.describe(): String =
  when (this) {
    is CommitTextCommand -> "CommitText(text=$text, newCursorPosition=$newCursorPosition)"
    is SetComposingTextCommand ->
      "SetComposingText(text=$text, newCursorPosition=$newCursorPosition)"
    is SetComposingRegionCommand -> "SetComposingRegion(start=$start, end=$end)"
    is SetSelectionCommand -> "SetSelection(start=$start, end=$end)"
    is FinishComposingTextCommand -> "FinishComposingText"
    is DeleteSurroundingTextCommand ->
      "DeleteSurroundingText(before=$lengthBeforeCursor, after=$lengthAfterCursor)"
    is DeleteSurroundingTextInCodePointsCommand ->
      "DeleteSurroundingTextInCodePoints(before=$lengthBeforeCursor, after=$lengthAfterCursor)"
    is BackspaceCommand -> "Backspace"
    is MoveCursorCommand -> "MoveCursor(amount=$amount)"
    else -> toString()
  }
