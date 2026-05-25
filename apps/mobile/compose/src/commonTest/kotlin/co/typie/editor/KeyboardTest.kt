package co.typie.editor

import androidx.compose.ui.input.key.Key as ComposeKey
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.platform.Clipboard
import co.typie.platform.ClipboardReadPayload
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest

class KeyboardTest {
  @Test
  fun shiftEnterDispatchesKeyEventSoUnitSelectionPolicyStaysInCore() = runTest {
    val binding =
      createBindings(Platform.Desktop).single {
        it.key == ComposeKey.Enter && it.modifiers == setOf(KeyModifier.Shift)
      }
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)

    val messages = with(binding) { editor.action(NoopClipboard) }

    assertEquals(
      listOf(Message.Key(FfiKeyEvent(FfiKey.Enter, InputModifiers(shift = true)))),
      messages,
    )
  }

  private object NoopClipboard : Clipboard {
    override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = true

    override suspend fun copy(text: String, mimeType: String): Boolean = true

    override suspend fun copyRichText(html: String, text: String): Boolean = true

    override suspend fun paste(): ClipboardReadPayload? = null
  }
}
