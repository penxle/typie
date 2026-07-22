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
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentMode
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
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
  fun navigationBindingsRevealCurrentSelectionHead() = runTest {
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)
    val navigationBindings =
      createBindings(Platform.Desktop)
        .filter { binding -> binding.action is EditorKeyBindingAction.Messages }
        .filter { binding -> binding.resolveMessages(editor).singleOrNull() is Message.Navigation }

    assertTrue(navigationBindings.isNotEmpty())
    navigationBindings.forEach { binding ->
      assertEquals(
        EditorBringIntoViewTarget.CurrentSelectionHead,
        binding.bringIntoViewTarget,
        "${binding.key} ${binding.modifiers}",
      )
    }
  }

  @Test
  fun shiftEnterDispatchesKeyEventSoUnitSelectionPolicyStaysInCore() = runTest {
    val binding =
      createBindings(Platform.Desktop).single {
        it.key == ComposeKey.Enter && it.modifiers == setOf(KeyModifier.Shift)
      }
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)

    val messages = binding.resolveMessages(editor)

    assertEquals(
      listOf(Message.Key(FfiKeyEvent(FfiKey.Enter, InputModifiers(shift = true)))),
      messages,
    )
  }

  @Test
  fun tabDispatchesKeyEventSoListFallbackPolicyStaysInCore() = runTest {
    val binding =
      createBindings(Platform.Desktop).single { it.key == ComposeKey.Tab && it.modifiers.isEmpty() }
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)

    assertEquals(listOf(Message.Key(FfiKeyEvent(FfiKey.Tab))), binding.resolveMessages(editor))
  }

  @Test
  fun shiftTabDispatchesKeyEventSoListFallbackPolicyStaysInCore() = runTest {
    val binding =
      createBindings(Platform.Desktop).single {
        it.key == ComposeKey.Tab && it.modifiers == setOf(KeyModifier.Shift)
      }
    val editor = Editor(FakeFfiEditor(), this, Dispatchers.Unconfined)

    assertEquals(
      listOf(Message.Key(FfiKeyEvent(FfiKey.Tab, InputModifiers(shift = true)))),
      binding.resolveMessages(editor),
    )
  }

  @Test
  fun pasteBindingsDeclarePasteActions() {
    val pasteBindings = createBindings(Platform.Desktop).filter { it.key == ComposeKey.V }

    assertEquals(
      EditorKeyBindingAction.Paste(IncomingContentMode.Rich),
      pasteBindings.single { it.modifiers == setOf(KeyModifier.Mod) }.action,
    )
    assertEquals(
      EditorKeyBindingAction.Paste(IncomingContentMode.PlainTextOnly),
      pasteBindings.single { it.modifiers == setOf(KeyModifier.Mod, KeyModifier.Shift) }.action,
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

    val messages = binding.resolveMessages(editor)

    assertEquals(listOf(Message.Navigation(NavigationOp.Move(expected, extend = true))), messages)
  }

  private suspend fun KeyBinding.resolveMessages(editor: Editor): List<Message> {
    val action = assertIs<EditorKeyBindingAction.Messages>(action)
    return action.messages(editor, NoopClipboard)
  }

  private object NoopClipboard : Clipboard {
    override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = true

    override suspend fun copy(text: String, mimeType: String): Boolean = true

    override suspend fun copyRichText(html: String, text: String): Boolean = true

    override suspend fun paste(): IncomingContentCandidates? = null
  }
}
