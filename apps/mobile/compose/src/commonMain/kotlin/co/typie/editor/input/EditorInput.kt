package co.typie.editor.input

import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.KeyInputModifierNode
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputModifierNode
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.platform.establishTextInputSession
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.EditorEventListener
import co.typie.editor.EditorState
import co.typie.editor.KeyBinding
import co.typie.editor.createBindings
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.matchesKeyBinding
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.editor.scroll.syncWithBringIntoView
import co.typie.ext.TextInputClient
import co.typie.ext.TextInputKey
import co.typie.ext.notifyTextInputFocusChanged
import co.typie.ext.registerTextInputClient
import co.typie.platform.Clipboard
import co.typie.platform.Platform
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch

internal fun Modifier.editorInput(
  session: DocumentEditingSession,
  uiState: EditorUiState,
  platform: Platform,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  enabled: Boolean = true,
  suppressSoftwareKeyboard: Boolean,
  clipboard: Clipboard,
): Modifier =
  this then
    EditorInputElement(
      session = session,
      uiState = uiState,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      enabled = enabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
      clipboard = clipboard,
    )

@OptIn(ExperimentalComposeUiApi::class)
internal expect suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  onEditCommand: (List<EditCommand>) -> Unit,
  focusedRectInRoot: () -> Rect?,
  textFieldRectInRoot: () -> Rect?,
  textClippingRectInRoot: () -> Rect?,
  suppressSoftwareKeyboard: Boolean,
  isSessionCurrent: () -> Boolean,
): PlatformTextInputMethodRequest

internal expect fun requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(): Boolean

internal fun rebindImeResync(
  previous: (() -> Unit)?,
  target: Editor,
  listener: EditorEventListener<EditorEvent.ImeResyncRequired>,
): () -> Unit {
  previous?.invoke()
  return target.on<EditorEvent.ImeResyncRequired>(listener)
}

internal fun shouldRestartEditorInputSession(
  previousEnabled: Boolean,
  enabled: Boolean,
  previousSuppressSoftwareKeyboard: Boolean,
  suppressSoftwareKeyboard: Boolean,
  restartOnSoftwareKeyboardSuppressionChange: Boolean =
    requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(),
): Boolean =
  previousEnabled != enabled ||
    (previousSuppressSoftwareKeyboard != suppressSoftwareKeyboard &&
      restartOnSoftwareKeyboardSuppressionChange)

internal fun requiresRawKeyTextFallback(platform: Platform): Boolean =
  platform == Platform.Android || platform == Platform.Desktop

internal fun toolbarInsertTextMessages(text: String, composing: Boolean): List<Message> =
  if (composing) {
    listOf(
      Message.TextInput(listOf(FlatImeOp.CommitAsIs)),
      Message.Insertion(InsertionOp.Text(text)),
    )
  } else {
    listOf(Message.Insertion(InsertionOp.Text(text)))
  }

private val COMPOSITION_COMMITTING_NAVIGATION_KEYS =
  setOf(
    ComposeKey.DirectionLeft,
    ComposeKey.DirectionRight,
    ComposeKey.DirectionUp,
    ComposeKey.DirectionDown,
    ComposeKey.MoveHome,
    ComposeKey.MoveEnd,
    ComposeKey.PageUp,
    ComposeKey.PageDown,
  )

// Android delivers hardware keys to the view only after the IME has declined them, so a
// blocked navigation key would leave the composition (and every following arrow) stuck.
// Other platforms deliver raw key events even while the IME is still consuming them
// (e.g. candidate navigation), so bindings stay blocked during composition there.
internal fun commitsCompositionBeforeKeyBinding(platform: Platform, key: ComposeKey): Boolean =
  platform == Platform.Android && key in COMPOSITION_COMMITTING_NAVIGATION_KEYS

internal fun fixedLocalCaretTextFieldRectInRoot(
  focusedRectInRoot: Rect?,
  textClippingRectInRoot: Rect?,
  fallbackRectInRoot: Rect?,
): Rect? {
  val focused = focusedRectInRoot ?: return fallbackRectInRoot
  val rightBoundary = textClippingRectInRoot?.right ?: fallbackRectInRoot?.right ?: focused.right
  return Rect(
    left = focused.left,
    top = focused.top,
    right = maxOf(focused.right, focused.left + 1f, rightBoundary),
    bottom = maxOf(focused.bottom, focused.top + 1f),
  )
}

private data class EditorInputElement(
  private val session: DocumentEditingSession,
  private val uiState: EditorUiState,
  private val platform: Platform,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val enabled: Boolean,
  private val suppressSoftwareKeyboard: Boolean,
  private val clipboard: Clipboard,
) : ModifierNodeElement<EditorInputNode>() {
  override fun create(): EditorInputNode =
    EditorInputNode(
      session = session,
      uiState = uiState,
      platform = platform,
      bringIntoViewRequests = bringIntoViewRequests,
      enabled = enabled,
      suppressSoftwareKeyboard = suppressSoftwareKeyboard,
      clipboard = clipboard,
    )

  override fun update(node: EditorInputNode) {
    val previousEditor = node.session.editor
    node.session = session
    node.uiState = uiState
    node.updatePlatform(platform)
    node.bringIntoViewRequests = bringIntoViewRequests
    node.clipboard = clipboard
    node.updateInputPolicy(enabled = enabled, suppressSoftwareKeyboard = suppressSoftwareKeyboard)
    if (node.session.editor !== previousEditor) {
      node.bindImeResync()
    }
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal class EditorInputNode(
  var session: DocumentEditingSession,
  var uiState: EditorUiState,
  var platform: Platform,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  enabled: Boolean,
  suppressSoftwareKeyboard: Boolean,
  var clipboard: Clipboard,
) : Modifier.Node(), FocusEventModifierNode, PlatformTextInputModifierNode, KeyInputModifierNode {
  private var focusedJob: Job? = null
  private var focused = false
  private var bindings = createBindings(platform)
    private set

  private var enabled = enabled
  private var suppressSoftwareKeyboard = suppressSoftwareKeyboard
  private val platformInputBridge = EditorPlatformInputBridge()
  private val editor: Editor
    get() = session.editor

  private var unsubscribeImeResync: (() -> Unit)? = null
  private var imeSessionGeneration = 0

  fun bindImeResync() {
    unsubscribeImeResync =
      rebindImeResync(unsubscribeImeResync, editor) { _, _ ->
        if (focused) {
          imeSessionGeneration += 1
          syncTextInputSession()
        }
      }
  }

  private fun <T : Job> submit(start: (Editor, CoroutineContext) -> T): T? = session.submit(start)

  fun updatePlatform(platform: Platform) {
    if (this.platform == platform) return

    this.platform = platform
    bindings = createBindings(platform)
    platformInputBridge.reset()
  }

  fun updateInputPolicy(enabled: Boolean, suppressSoftwareKeyboard: Boolean) {
    val shouldRestart =
      shouldRestartEditorInputSession(
        previousEnabled = this.enabled,
        enabled = enabled,
        previousSuppressSoftwareKeyboard = this.suppressSoftwareKeyboard,
        suppressSoftwareKeyboard = suppressSoftwareKeyboard,
      )
    if (this.enabled == enabled && this.suppressSoftwareKeyboard == suppressSoftwareKeyboard) {
      return
    }

    this.enabled = enabled
    this.suppressSoftwareKeyboard = suppressSoftwareKeyboard
    if (shouldRestart) {
      syncTextInputSession()
    }
  }

  private fun dispatch(
    messages: List<Message>,
    bringIntoViewTarget: EditorBringIntoViewTarget? =
      EditorBringIntoViewTarget.CurrentSelectionHead,
  ) {
    if (messages.isEmpty()) return
    submit { sessionEditor, context ->
      sessionEditor.scope.launch(context) {
        sessionEditor.awaitWithBringIntoView(bringIntoViewRequests) {
          messages.forEach(::enqueue)
          beforeCommit { bringIntoViewTarget?.let { target -> bringIntoView(target) } }
        }
      }
    }
  }

  // The coalescer worker lives in the attach-epoch coroutineScope, which is
  // cancelled whenever the node detaches (movable content moves reattach the
  // same node instance), so it must be rebuilt per attach instead of lazily.
  private var bindingCoalescer: EditorKeyBindingCoalescer? = null

  override fun onAttach() {
    bindingCoalescer =
      EditorKeyBindingCoalescer(
        scope = coroutineScope,
        resolveMessages = { binding, clipboard -> with(binding) { editor.action(clipboard) } },
        dispatch = { messages, bringIntoViewTarget ->
          dispatchBindingMessages(messages = messages, bringIntoViewTarget = bringIntoViewTarget)
        },
      )
    bindImeResync()
  }

  private fun dispatchBinding(binding: KeyBinding, clipboard: Clipboard) {
    val coalescer = bindingCoalescer ?: return
    if (binding.coalescible) {
      submit { _, localEdit -> coalescer.submit(binding, clipboard, localEdit) }
    } else {
      coroutineScope.launch { coalescer.submit(binding, clipboard).await() }
    }
  }

  private suspend fun dispatchBindingMessages(
    messages: List<Message>,
    bringIntoViewTarget: EditorBringIntoViewTarget?,
  ): EditorState? {
    if (messages.isEmpty()) return null
    return editor.awaitWithBringIntoView(bringIntoViewRequests) {
      messages.forEach(::enqueue)
      beforeCommit { bringIntoViewTarget?.let { target -> bringIntoView(target) } }
    }
  }

  private fun dispatchSync(
    messages: List<Message>,
    bringIntoViewTarget: EditorBringIntoViewTarget? =
      EditorBringIntoViewTarget.CurrentSelectionHead,
  ): EditorState? {
    if (messages.isEmpty()) return null
    return editor.syncWithBringIntoView(bringIntoViewRequests) {
      messages.forEach(::enqueue)
      beforeCommit { bringIntoViewTarget?.let { target -> bringIntoView(target) } }
    }
  }

  private val textInputClient =
    object : TextInputClient {
      override val hasActiveComposition: Boolean
        get() = editor.ime?.composing != null

      override fun requestFocus() {
        editor.focus()
      }

      override fun insertText(text: String): Boolean {
        recordToolbarInput("insertText", text)
        dispatch(toolbarInsertTextMessages(text, editor.ime?.composing != null))
        return true
      }

      override fun commitText(text: String) {
        recordToolbarInput("commitText", text)
        if (text == "\n") {
          dispatch(listOf(Message.Insertion(InsertionOp.Text(text))))
        } else {
          dispatch(listOf(Message.TextInput(listOf(FlatImeOp.Compose(text), FlatImeOp.CommitAsIs))))
        }
      }

      override fun setComposingText(text: String) {
        recordToolbarInput("setComposingText", text)
        dispatch(listOf(Message.TextInput(listOf(FlatImeOp.Compose(text)))))
      }

      override fun finishComposition() {
        recordToolbarInput("finishComposition")
        dispatch(listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs))))
      }

      override fun pressKey(key: TextInputKey): Boolean {
        recordToolbarInput("pressKey", key.name)
        val ffiKey =
          when (key) {
            TextInputKey.Enter -> FfiKey.Enter
            TextInputKey.Backspace -> FfiKey.Backspace
          }
        dispatch(listOf(Message.Key(FfiKeyEvent(ffiKey))))
        return true
      }

      override fun dismiss() {
        editor.blur()
      }
    }

  private fun recordHardwareKey(
    event: KeyEvent,
    stage: String,
    matchedBinding: Boolean,
    blockedByComposition: Boolean,
    consumed: Boolean,
    text: String? = null,
  ) {
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.HardwareKey(
        seq = seq,
        t = t,
        key = event.key.toString(),
        stage = stage,
        matchedBinding = matchedBinding,
        blockedByComposition = blockedByComposition,
        consumed = consumed,
        text = text,
      )
    }
  }

  private fun recordToolbarInput(method: String, args: String? = null) {
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.ToolbarInput(seq = seq, t = t, method = method, args = args)
    }
  }

  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (!enabled || event.type != KeyEventType.KeyDown) return false
    val binding = bindings.find { matchesKeyBinding(it, platform, event) }
    if (binding != null) {
      val composing = editor.ime?.composing != null
      if (composing && !commitsCompositionBeforeKeyBinding(platform, event.key)) {
        recordHardwareKey(
          event = event,
          stage = "onKeyEvent",
          matchedBinding = true,
          blockedByComposition = true,
          consumed = true,
        )
        return true
      }
      if (composing) {
        dispatchSync(
          listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs))),
          bringIntoViewTarget = null,
        )
      }
      if (platformInputBridge.shouldConsumeKeyEvent(event)) {
        recordHardwareKey(
          event = event,
          stage = "onKeyEvent",
          matchedBinding = true,
          blockedByComposition = false,
          consumed = true,
        )
        return true
      }
      recordHardwareKey(
        event = event,
        stage = "onKeyEvent",
        matchedBinding = true,
        blockedByComposition = false,
        consumed = true,
      )
      dispatchBinding(binding, clipboard)
      return true
    }
    if (!requiresRawKeyTextFallback(platform)) {
      return false
    }

    val cp = event.utf16CodePoint
    if (cp > 0xFFFF) {
      val text =
        charArrayOf(
            (((cp - 0x10000) ushr 10) + 0xD800).toChar(),
            (((cp - 0x10000) and 0x3FF) + 0xDC00).toChar(),
          )
          .concatToString()
      if (editor.ime?.composing != null) {
        recordHardwareKey(
          event = event,
          stage = "onKeyEvent",
          matchedBinding = false,
          blockedByComposition = true,
          consumed = true,
          text = text,
        )
        return true
      }
      recordHardwareKey(
        event = event,
        stage = "onKeyEvent",
        matchedBinding = false,
        blockedByComposition = false,
        consumed = true,
        text = text,
      )
      dispatch(listOf(Message.Insertion(InsertionOp.Text(text))))
      return true
    }

    val ch = cp.toChar()
    if (!ch.isDefined() || ch.isISOControl() || ch.isSurrogate()) return false

    if (editor.ime?.composing != null) {
      recordHardwareKey(
        event = event,
        stage = "onKeyEvent",
        matchedBinding = false,
        blockedByComposition = true,
        consumed = true,
        text = ch.toString(),
      )
      return true
    }
    recordHardwareKey(
      event = event,
      stage = "onKeyEvent",
      matchedBinding = false,
      blockedByComposition = false,
      consumed = true,
      text = ch.toString(),
    )
    dispatch(listOf(Message.Insertion(InsertionOp.Text(ch.toString()))))
    return true
  }

  override fun onPreKeyEvent(event: KeyEvent): Boolean {
    if (!enabled || event.type != KeyEventType.KeyDown) return false
    val binding = bindings.find { matchesKeyBinding(it, platform, event) } ?: return false
    if (editor.ime?.composing != null && !commitsCompositionBeforeKeyBinding(platform, event.key)) {
      recordHardwareKey(
        event = event,
        stage = "onPreKeyEvent",
        matchedBinding = true,
        blockedByComposition = true,
        consumed = true,
      )
      return true
    }
    val consumed =
      platformInputBridge.onPreKeyEvent(
        event = event,
        editorState = { editor.state },
        inputCoroutineScope = coroutineScope,
        bindingMessages = { with(binding) { editor.action(clipboard) } },
        commit = { messages ->
          dispatchBindingMessages(
            messages = messages,
            bringIntoViewTarget = binding.bringIntoViewTarget,
          )
        },
      )
    recordHardwareKey(
      event = event,
      stage = "onPreKeyEvent",
      matchedBinding = true,
      blockedByComposition = false,
      consumed = consumed,
    )
    return consumed
  }

  override fun onFocusEvent(focusState: FocusState) {
    val nextFocused = focusState.isFocused
    if (focused == nextFocused) {
      return
    }

    focused = nextFocused
    syncTextInputSession()
  }

  private fun syncTextInputSession() {
    val sessionEnabled = focused && enabled
    val generationAtStart = imeSessionGeneration
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.Session(seq = seq, t = t, event = if (sessionEnabled) "start" else "stop")
    }
    if (!sessionEnabled && editor.ime?.composing != null) {
      dispatchSync(
        listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs))),
        bringIntoViewTarget = null,
      )
    }
    focusedJob?.cancel()
    focusedJob = null
    platformInputBridge.reset()
    notifyTextInputFocusChanged(this, sessionEnabled)
    registerTextInputClient(this, if (sessionEnabled) textInputClient else null)
    focusedJob =
      if (sessionEnabled) {
        coroutineScope.launch {
          val uninstallPlatformSessionEffects =
            platformInputBridge.installSessionEffects(
              cursor = { editor.cursor },
              viewportTransform = { uiState.resolveViewportTransform(editor.pageSizes) },
              dispatch = { messages -> dispatch(messages) },
            )
          try {
            establishTextInputSession {
              val request =
                createEditorInputRequest(
                  editor = editor,
                  bringIntoViewRequests = bringIntoViewRequests,
                  onEditCommand = { commands ->
                    val preState = editor.state
                    val intercepted =
                      platformInputBridge.interceptEditCommands(
                        commands = commands,
                        state = preState,
                      )
                    val messages =
                      intercepted
                        ?: EditorImeCommandNormalizer.normalize(
                          commands = commands,
                          ime = preState.ime,
                        )
                    val postState = dispatchSync(messages)
                    if (postState != null) {
                      platformInputBridge.onImeMessagesCommitted(
                        messages = messages,
                        preState = preState,
                        postState = postState,
                      )
                    }
                    editor.inputRecorder?.record { seq, t ->
                      RecordedInputEntry.EditCommands(
                        seq = seq,
                        t = t,
                        commands = commands.map { it.describe() },
                        decision = classifyBridgeRoute(intercepted),
                        messages = messages,
                        imeBefore = preState.ime,
                        imeAfter = postState?.ime,
                      )
                    }
                  },
                  focusedRectInRoot = { uiState.cursorRectInRoot(editor.cursor) },
                  textFieldRectInRoot = uiState::editorRectInRoot,
                  textClippingRectInRoot = uiState::textClippingRectInRoot,
                  suppressSoftwareKeyboard = suppressSoftwareKeyboard,
                  isSessionCurrent = { imeSessionGeneration == generationAtStart },
                )
              launch {
                notifyImeStateChanged(editor)
                // tickIme covers window text/composing changes that leave the
                // selection untouched (e.g. remote edits after the cursor), so
                // extracted-text monitors never go stale.
                snapshotFlow { Triple(editor.selection, editor.cursor, editor.tickIme) }
                  .distinctUntilChanged()
                  .drop(1) // initial emission already handled above
                  .collect { notifyImeStateChanged(editor) }
              }

              startInputMethod(request)
            }
          } finally {
            uninstallPlatformSessionEffects()
          }
        }
      } else {
        null
      }
  }

  override fun onDetach() {
    unsubscribeImeResync?.invoke()
    unsubscribeImeResync = null
    bindingCoalescer = null
    focused = false
    platformInputBridge.reset()
    notifyTextInputFocusChanged(this, false)
    registerTextInputClient(this, null)
    focusedJob?.cancel()
    super.onDetach()
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal expect fun PlatformTextInputSessionScope.notifyImeStateChanged(editor: Editor)
