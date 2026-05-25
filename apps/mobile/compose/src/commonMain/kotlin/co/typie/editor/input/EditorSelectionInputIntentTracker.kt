package co.typie.editor.input

import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp

internal sealed interface EditorSelectionInputDecision {
  data object DropNativeSelectionCommand : EditorSelectionInputDecision

  data class ReplayNativeCommandAsAppOwnedNavigation(val messages: List<Message>) :
    EditorSelectionInputDecision
}

internal data class EditorSelectionInputDispatchToken(val value: Long)

internal class EditorSelectionInputIntentTracker(private val staleTimeoutMillis: Long = 750L) {
  private var nextToken = 0L
  private var lifecycle: SelectionInputLifecycle = SelectionInputLifecycle.Idle

  fun reset() {
    lifecycle = SelectionInputLifecycle.Idle
  }

  fun recordAppOwnedDispatch(
    messages: List<Message>,
    preState: EditorState,
    nowMillis: Long,
  ): EditorSelectionInputDispatchToken? {
    expireStale(nowMillis)
    val move = messages.singleNavigationMoveOrNull()
    if (move == null) {
      reset()
      return null
    }

    val token = nextDispatchToken()
    lifecycle =
      SelectionInputLifecycle.InFlight(
        token = token,
        intent =
          AppOwnedSelectionDispatchIntent(
            movement = move.movement,
            extend = move.extend,
            preVersion = preState.version,
            preSelection = preState.ime?.selection,
            staleAtMillis = nowMillis + staleTimeoutMillis,
          ),
        active = lifecycle.activeIntentOrNull(),
      )
    return token
  }

  fun cancelAppOwnedDispatch(token: EditorSelectionInputDispatchToken) {
    val current = lifecycle
    if (current is SelectionInputLifecycle.InFlight && current.token == token) {
      lifecycle =
        current.active?.let(SelectionInputLifecycle::Active) ?: SelectionInputLifecycle.Idle
    }
  }

  fun recordAppOwnedCommit(
    token: EditorSelectionInputDispatchToken,
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
    nowMillis: Long,
  ) {
    val current = lifecycle
    if (current !is SelectionInputLifecycle.InFlight || current.token != token) {
      return
    }

    val move = messages.singleNavigationMoveOrNull()
    if (move == null) {
      lifecycle =
        current.active?.let(SelectionInputLifecycle::Active) ?: SelectionInputLifecycle.Idle
      return
    }

    lifecycle =
      SelectionInputLifecycle.Active(
        AppOwnedSelectionCommitIntent(
          movement = move.movement,
          extend = move.extend,
          preVersion = preState.version,
          postVersion = postState.version,
          expectedNativeProjection = postState.ime?.selection,
          staleAtMillis = nowMillis + staleTimeoutMillis,
        )
      )
  }

  fun classifyNativeSelectionCommands(
    commands: List<EditCommand>,
    state: EditorState,
    nowMillis: Long,
  ): EditorSelectionInputDecision? {
    expireStale(nowMillis)

    val projection = commands.projectSelectionOnlyCommand(state.ime)
    val target =
      when (projection) {
        null -> {
          reset()
          return null
        }

        SelectionOnlyEditCommandProjection.MissingIme -> {
          reset()
          return null
        }

        is SelectionOnlyEditCommandProjection.Target -> projection.range
      }

    val active = lifecycle.activeIntentOrNull()
    if (
      active != null &&
        state.version == active.postVersion &&
        target == active.expectedNativeProjection
    ) {
      lifecycle = lifecycle.withActive(active.refreshed(nowMillis, staleTimeoutMillis))
      return EditorSelectionInputDecision.DropNativeSelectionCommand
    }

    val current = lifecycle
    if (current is SelectionInputLifecycle.InFlight) {
      if (
        state.version == current.intent.preVersion &&
          current.intent.matchesNativeReplay(to = target)
      ) {
        lifecycle = current.copy(intent = current.intent.refreshed(nowMillis, staleTimeoutMillis))
        return EditorSelectionInputDecision.DropNativeSelectionCommand
      }

      return EditorSelectionInputDecision.DropNativeSelectionCommand
    }

    val intent = lifecycle.activeIntentOrNull() ?: return null
    if (state.version != intent.postVersion) {
      reset()
      return null
    }

    val selection = state.ime?.selection
    if (!intent.matchesNativeReplay(from = selection, to = target)) {
      reset()
      return null
    }

    lifecycle = SelectionInputLifecycle.Replaying(intent.refreshed(nowMillis, staleTimeoutMillis))
    return EditorSelectionInputDecision.ReplayNativeCommandAsAppOwnedNavigation(
      listOf(Message.Navigation(NavigationOp.Move(intent.movement, intent.extend)))
    )
  }

  fun recordImeMessagesCommitted(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
    nowMillis: Long,
  ) {
    val current = lifecycle
    if (current !is SelectionInputLifecycle.Replaying) {
      if (messages.isNotEmpty()) reset()
      return
    }

    lifecycle =
      SelectionInputLifecycle.Active(
        current.intent.copy(
          preVersion = preState.version,
          postVersion = postState.version,
          expectedNativeProjection = postState.ime?.selection,
          staleAtMillis = nowMillis + staleTimeoutMillis,
        )
      )
  }

  private fun expireStale(nowMillis: Long) {
    lifecycle =
      when (val current = lifecycle) {
        SelectionInputLifecycle.Idle -> SelectionInputLifecycle.Idle
        is SelectionInputLifecycle.Active ->
          if (nowMillis >= current.intent.staleAtMillis) {
            SelectionInputLifecycle.Idle
          } else {
            current
          }

        is SelectionInputLifecycle.InFlight -> {
          val active = current.active?.takeUnless { nowMillis >= it.staleAtMillis }
          if (nowMillis >= current.intent.staleAtMillis) {
            active?.let(SelectionInputLifecycle::Active) ?: SelectionInputLifecycle.Idle
          } else {
            current.copy(active = active)
          }
        }

        is SelectionInputLifecycle.Replaying ->
          if (nowMillis >= current.intent.staleAtMillis) {
            SelectionInputLifecycle.Idle
          } else {
            current
          }
      }
  }

  private fun nextDispatchToken(): EditorSelectionInputDispatchToken {
    val token = EditorSelectionInputDispatchToken(nextToken)
    nextToken += 1
    return token
  }
}

private sealed interface SelectionInputLifecycle {
  data object Idle : SelectionInputLifecycle

  data class Active(val intent: AppOwnedSelectionCommitIntent) : SelectionInputLifecycle

  data class InFlight(
    val token: EditorSelectionInputDispatchToken,
    val intent: AppOwnedSelectionDispatchIntent,
    val active: AppOwnedSelectionCommitIntent?,
  ) : SelectionInputLifecycle

  data class Replaying(val intent: AppOwnedSelectionCommitIntent) : SelectionInputLifecycle
}

private fun SelectionInputLifecycle.activeIntentOrNull(): AppOwnedSelectionCommitIntent? =
  when (this) {
    SelectionInputLifecycle.Idle -> null
    is SelectionInputLifecycle.Active -> intent
    is SelectionInputLifecycle.InFlight -> active
    is SelectionInputLifecycle.Replaying -> intent
  }

private fun SelectionInputLifecycle.withActive(
  active: AppOwnedSelectionCommitIntent
): SelectionInputLifecycle =
  when (this) {
    SelectionInputLifecycle.Idle -> SelectionInputLifecycle.Active(active)
    is SelectionInputLifecycle.Active -> copy(intent = active)
    is SelectionInputLifecycle.InFlight -> copy(active = active)
    is SelectionInputLifecycle.Replaying -> copy(intent = active)
  }

private data class AppOwnedSelectionDispatchIntent(
  val movement: Movement,
  val extend: Boolean,
  val preVersion: Long,
  val preSelection: ImeRange?,
  val staleAtMillis: Long,
) {
  fun refreshed(nowMillis: Long, staleTimeoutMillis: Long): AppOwnedSelectionDispatchIntent =
    copy(staleAtMillis = nowMillis + staleTimeoutMillis)

  fun matchesNativeReplay(to: ImeRange): Boolean {
    val from = preSelection ?: return false
    if (to == from) return true
    val direction = movement.directionOrNull() ?: return false
    return targetMovesInDirection(from = from, to = to, direction = direction)
  }
}

private data class AppOwnedSelectionCommitIntent(
  val movement: Movement,
  val extend: Boolean,
  val preVersion: Long,
  val postVersion: Long,
  val expectedNativeProjection: ImeRange?,
  val staleAtMillis: Long,
) {
  fun refreshed(nowMillis: Long, staleTimeoutMillis: Long): AppOwnedSelectionCommitIntent =
    copy(staleAtMillis = nowMillis + staleTimeoutMillis)

  fun matchesNativeReplay(from: ImeRange?, to: ImeRange): Boolean {
    from ?: return false
    val direction = movement.directionOrNull() ?: return false
    return targetMovesInDirection(from = from, to = to, direction = direction)
  }
}

private fun targetMovesInDirection(from: ImeRange, to: ImeRange, direction: Direction): Boolean =
  when (direction) {
    Direction.Backward -> to.start < from.start || to.end < from.end
    Direction.Forward -> to.start > from.start || to.end > from.end
  }

private data class NavigationMoveIntent(val movement: Movement, val extend: Boolean)

private fun List<Message>.singleNavigationMoveOrNull(): NavigationMoveIntent? {
  val message = singleOrNull() as? Message.Navigation ?: return null
  val move = message.op as? NavigationOp.Move ?: return null
  return NavigationMoveIntent(movement = move.movement, extend = move.extend)
}

private fun Movement.directionOrNull(): Direction? =
  when (this) {
    is Movement.Block -> direction
    is Movement.Document -> direction
    is Movement.Grapheme -> direction
    is Movement.Line -> direction
    is Movement.Page -> direction
    is Movement.Sentence -> direction
    is Movement.Word -> direction
  }
