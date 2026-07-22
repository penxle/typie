package co.typie.editor.input

import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.EditorState
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorSelectionInputIntentTrackerTest {
  @Test
  fun `native selection echo is consumed only when committed post selection and version match`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)

    tracker.recordCommittedAppOwnedSelection(
      messages = listOf(move(direction = Direction.Backward, extend = true)),
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 0L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(17, 18)),
        state = state(version = 2L, selection = ImeRange(17, 18)),
        nowMillis = 300L,
      ),
    )

    assertNull(
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(17, 18)),
        state = state(version = 3L, selection = ImeRange(17, 18)),
        nowMillis = 301L,
      )
    )
  }

  @Test
  fun `native replay from active hardware selection intent is rewritten to original movement`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val expected = listOf(move(direction = Direction.Backward, extend = true))

    tracker.recordCommittedAppOwnedSelection(
      messages = expected,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 0L,
    )

    assertEquals(
      EditorSelectionInputDecision.ReplayNativeCommandAsAppOwnedNavigation(expected),
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(16, 17)),
        state = state(version = 2L, selection = ImeRange(17, 18)),
        nowMillis = 100L,
      ),
    )

    tracker.recordImeMessagesCommitted(
      messages = expected,
      preState = state(version = 2L, selection = ImeRange(17, 18)),
      postState = state(version = 3L, selection = ImeRange(16, 18)),
      nowMillis = 101L,
    )

    assertEquals(
      EditorSelectionInputDecision.ReplayNativeCommandAsAppOwnedNavigation(expected),
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(15, 17)),
        state = state(version = 3L, selection = ImeRange(16, 18)),
        nowMillis = 200L,
      ),
    )
  }

  @Test
  fun `native replay uses committed binding movement instead of hard coded grapheme`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val expected =
      listOf(
        Message.Navigation(
          NavigationOp.Move(Movement.Line(Direction.Forward, Axis.Horizontal), true)
        )
      )

    tracker.recordCommittedAppOwnedSelection(
      messages = expected,
      preState = state(version = 1L, selection = ImeRange(10, 18)),
      postState = state(version = 2L, selection = ImeRange(11, 18)),
      nowMillis = 0L,
    )

    assertEquals(
      EditorSelectionInputDecision.ReplayNativeCommandAsAppOwnedNavigation(expected),
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(12, 18)),
        state = state(version = 2L, selection = ImeRange(11, 18)),
        nowMillis = 100L,
      ),
    )
  }

  @Test
  fun `native selection replay before app owned commit is consumed while commit is in flight`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val expected =
      listOf(
        Message.Navigation(
          NavigationOp.Move(Movement.Line(Direction.Backward, Axis.Horizontal), true)
        )
      )

    val token =
      tracker.recordAppOwnedDispatch(
        messages = expected,
        preState = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 0L,
      ) ?: error("expected selection dispatch token")

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(17, 18)),
        state = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 50L,
      ),
    )

    tracker.recordAppOwnedCommit(
      token = token,
      messages = expected,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(0, 18)),
      nowMillis = 100L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(0, 18)),
        state = state(version = 2L, selection = ImeRange(0, 18)),
        nowMillis = 150L,
      ),
    )
  }

  @Test
  fun `timeout cleans stale intent but does not make mismatched command an echo`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)

    tracker.recordCommittedAppOwnedSelection(
      messages = listOf(move(direction = Direction.Backward, extend = true)),
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 0L,
    )

    assertNull(
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(18, 19)),
        state = state(version = 2L, selection = ImeRange(17, 18)),
        nowMillis = 50L,
      )
    )

    tracker.recordCommittedAppOwnedSelection(
      messages = listOf(move(direction = Direction.Backward, extend = true)),
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 0L,
    )

    assertNull(
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(17, 18)),
        state = state(version = 2L, selection = ImeRange(17, 18)),
        nowMillis = 1_001L,
      )
    )
  }

  @Test
  fun `native selection without active app owned intent passes through`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)

    assertNull(
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(1, 4)),
        state = state(version = 1L, selection = ImeRange(1, 1)),
        nowMillis = 0L,
      )
    )
  }

  @Test
  fun `stale app owned commit cannot clear newer in flight intent`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val first = listOf(move(direction = Direction.Backward, extend = true))
    val second = listOf(move(direction = Direction.Forward, extend = false))

    val firstToken =
      tracker.recordAppOwnedDispatch(
        messages = first,
        preState = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 0L,
      ) ?: error("expected first dispatch token")
    val secondToken =
      tracker.recordAppOwnedDispatch(
        messages = second,
        preState = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 1L,
      ) ?: error("expected second dispatch token")

    tracker.recordAppOwnedCommit(
      token = firstToken,
      messages = first,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 2L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(19, 19)),
        state = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 3L,
      ),
    )

    tracker.recordAppOwnedCommit(
      token = secondToken,
      messages = second,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 3L, selection = ImeRange(19, 19)),
      nowMillis = 4L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(19, 19)),
        state = state(version = 3L, selection = ImeRange(19, 19)),
        nowMillis = 5L,
      ),
    )
  }

  @Test
  fun `stale native echo cannot clear newer in flight intent`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val first = listOf(move(direction = Direction.Backward, extend = true))
    val second = listOf(move(direction = Direction.Forward, extend = false))

    val firstToken =
      tracker.recordAppOwnedDispatch(
        messages = first,
        preState = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 0L,
      ) ?: error("expected first dispatch token")
    val secondToken =
      tracker.recordAppOwnedDispatch(
        messages = second,
        preState = state(version = 1L, selection = ImeRange(18, 18)),
        nowMillis = 1L,
      ) ?: error("expected second dispatch token")

    tracker.recordAppOwnedCommit(
      token = firstToken,
      messages = first,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 2L, selection = ImeRange(17, 18)),
      nowMillis = 2L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(17, 18)),
        state = state(version = 2L, selection = ImeRange(17, 18)),
        nowMillis = 3L,
      ),
    )

    tracker.recordAppOwnedCommit(
      token = secondToken,
      messages = second,
      preState = state(version = 1L, selection = ImeRange(18, 18)),
      postState = state(version = 3L, selection = ImeRange(19, 19)),
      nowMillis = 4L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(19, 19)),
        state = state(version = 3L, selection = ImeRange(19, 19)),
        nowMillis = 5L,
      ),
    )
  }

  @Test
  fun `late non navigation dispatch cannot clear newer in flight intent`() {
    val tracker = EditorSelectionInputIntentTracker(staleTimeoutMillis = 1_000L)
    val messages = listOf(move(direction = Direction.Forward, extend = false))
    val preState = state(version = 1L, selection = ImeRange(18, 18))

    val token =
      tracker.recordAppOwnedDispatch(messages = messages, preState = preState, nowMillis = 0L)
        ?: error("expected selection dispatch token")
    tracker.recordAppOwnedDispatch(messages = emptyList(), preState = preState, nowMillis = 1L)

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(19, 19)),
        state = preState,
        nowMillis = 2L,
      ),
    )

    tracker.recordAppOwnedCommit(
      token = token,
      messages = messages,
      preState = preState,
      postState = state(version = 2L, selection = ImeRange(19, 19)),
      nowMillis = 3L,
    )

    assertEquals(
      EditorSelectionInputDecision.DropNativeSelectionCommand,
      tracker.classifyNativeSelectionCommands(
        commands = listOf(SetSelectionCommand(19, 19)),
        state = state(version = 2L, selection = ImeRange(19, 19)),
        nowMillis = 4L,
      ),
    )
  }

  private fun move(direction: Direction, extend: Boolean): Message =
    Message.Navigation(NavigationOp.Move(Movement.Grapheme(direction), extend))

  private fun EditorSelectionInputIntentTracker.recordCommittedAppOwnedSelection(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
    nowMillis: Long,
  ) {
    val token =
      recordAppOwnedDispatch(messages = messages, preState = preState, nowMillis = nowMillis)
        ?: error("expected selection dispatch token")
    recordAppOwnedCommit(
      token = token,
      messages = messages,
      preState = preState,
      postState = postState,
      nowMillis = nowMillis,
    )
  }

  private fun state(version: Long, selection: ImeRange): EditorState =
    EditorState.Initial.copy(
      version = version,
      ime =
        Ime(text = "abcdefghijklmnopqrst", windowStart = 0, selection = selection, composing = null),
    )
}
