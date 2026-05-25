package co.typie.editor

import androidx.compose.ui.input.key.Key as ComposeKey
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.platform.Clipboard
import co.typie.platform.ClipboardReadPayload
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest

class KeyboardTest {
  @Test
  fun shiftModUpAndDownDispatchDocumentSelectionNavigation() = runTest {
    assertNavigationBinding(
      key = ComposeKey.DirectionUp,
      modifiers = setOf(KeyModifier.Mod, KeyModifier.Shift),
      expected = Movement.Document(Direction.Backward),
    )
    assertNavigationBinding(
      key = ComposeKey.DirectionDown,
      modifiers = setOf(KeyModifier.Mod, KeyModifier.Shift),
      expected = Movement.Document(Direction.Forward),
    )
  }

  @Test
  fun shiftedNavigationBindingsCoverExistingNonShiftNavigationFamilies() = runTest {
    assertNavigationBinding(
      key = ComposeKey.DirectionLeft,
      modifiers = setOf(KeyModifier.Ctrl, KeyModifier.Shift),
      expected = Movement.Word(Direction.Backward),
    )
    assertNavigationBinding(
      key = ComposeKey.DirectionRight,
      modifiers = setOf(KeyModifier.Ctrl, KeyModifier.Shift),
      expected = Movement.Word(Direction.Forward),
    )
    assertNavigationBinding(
      key = ComposeKey.DirectionUp,
      modifiers = setOf(KeyModifier.Alt, KeyModifier.Shift),
      expected = Movement.Sentence(Direction.Backward),
    )
    assertNavigationBinding(
      key = ComposeKey.DirectionDown,
      modifiers = setOf(KeyModifier.Alt, KeyModifier.Shift),
      expected = Movement.Sentence(Direction.Forward),
    )
    assertNavigationBinding(
      key = ComposeKey.MoveHome,
      modifiers = setOf(KeyModifier.Shift),
      expected = Movement.Line(Direction.Backward, Axis.Horizontal),
    )
    assertNavigationBinding(
      key = ComposeKey.MoveHome,
      modifiers = setOf(KeyModifier.Ctrl, KeyModifier.Shift),
      expected = Movement.Document(Direction.Backward),
    )
    assertNavigationBinding(
      key = ComposeKey.MoveEnd,
      modifiers = setOf(KeyModifier.Shift),
      expected = Movement.Line(Direction.Forward, Axis.Horizontal),
    )
    assertNavigationBinding(
      key = ComposeKey.MoveEnd,
      modifiers = setOf(KeyModifier.Ctrl, KeyModifier.Shift),
      expected = Movement.Document(Direction.Forward),
    )
    assertNavigationBinding(
      key = ComposeKey.PageUp,
      modifiers = setOf(KeyModifier.Shift),
      expected = Movement.Page(Direction.Backward),
    )
    assertNavigationBinding(
      key = ComposeKey.PageDown,
      modifiers = setOf(KeyModifier.Shift),
      expected = Movement.Page(Direction.Forward),
    )
  }

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

  private suspend fun TestScope.assertNavigationBinding(
    key: ComposeKey,
    modifiers: Set<KeyModifier>,
    expected: Movement,
  ) {
    val binding =
      createBindings(Platform.Desktop).single { it.key == key && it.modifiers == modifiers }
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)

    val messages = with(binding) { editor.action(NoopClipboard) }

    assertEquals(listOf(Message.Navigation(NavigationOp.Move(expected, extend = true))), messages)
  }

  private object NoopClipboard : Clipboard {
    override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = true

    override suspend fun copy(text: String, mimeType: String): Boolean = true

    override suspend fun copyRichText(html: String, text: String): Boolean = true

    override suspend fun paste(): ClipboardReadPayload? = null
  }
}
