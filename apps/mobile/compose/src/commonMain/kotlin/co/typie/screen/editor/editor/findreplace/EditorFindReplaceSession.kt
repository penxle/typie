package co.typie.screen.editor.editor.findreplace

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.storage.Preference
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Stable
internal class EditorFindReplaceSession(
  val active: Boolean,
  val findText: String,
  val replaceText: String,
  val matchWholeWord: Boolean,
  val matchCount: Int,
  val activeMatchNumber: Int?,
  val searchInputFocusRequest: Int,
  private val onOpen: () -> Unit,
  val close: () -> Unit,
  val updateFindText: (String) -> Unit,
  val updateReplaceText: (String) -> Unit,
  val updateMatchWholeWord: (Boolean) -> Unit,
  val findPrevious: () -> Unit,
  val findNext: () -> Unit,
  val replace: () -> Unit,
  val replaceAll: () -> Unit,
) {
  val hasMatches: Boolean
    get() = matchCount > 0

  val canReplace: Boolean
    get() = findText.isNotEmpty() && hasMatches

  fun open() = onOpen()
}

private data class FindReplaceSearchInput(val query: String, val matchWholeWord: Boolean)

@Composable
internal fun rememberEditorFindReplaceSession(
  documentLocked: Boolean,
  editingSession: DocumentEditingSession?,
  editorState: EditorState,
  bringIntoViewRequests: EditorBringIntoViewRequests,
): EditorFindReplaceSession {
  val state = remember { FindReplaceSessionController() }
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  val matchWholeWord = Preference.searchMatchWholeWord
  val editor = editingSession?.editor

  DisposableEffect(editor) { onDispose { editor?.clearFindReplaceRanges() } }

  LaunchedEffect(state.active, editor) {
    if (state.active) {
      editor?.installFindReplaceDecorations()
    }
  }

  LaunchedEffect(
    state.active,
    editor,
    editorState.documentRevision,
    state.findText,
    matchWholeWord,
  ) {
    state.runSearch(
      editor = editor,
      matchWholeWord = matchWholeWord,
      bringIntoViewRequests = bringIntoViewRequests,
    )
  }

  return EditorFindReplaceSession(
    active = state.active,
    findText = state.findText,
    replaceText = state.replaceText,
    matchWholeWord = matchWholeWord,
    matchCount = state.matches.size,
    activeMatchNumber = state.activeIndex?.let { it + 1 },
    searchInputFocusRequest = state.searchInputFocusRequest,
    onOpen = { scope.launch { state.open(editor) } },
    close = { scope.launch { state.close(editor) } },
    updateFindText = state::updateFindText,
    updateReplaceText = state::updateReplaceText,
    updateMatchWholeWord = { enabled -> Preference.searchMatchWholeWord = enabled },
    findPrevious = {
      scope.launch {
        state.findBy(offset = -1, editor = editor, bringIntoViewRequests = bringIntoViewRequests)
      }
    },
    findNext = {
      scope.launch {
        state.findBy(offset = 1, editor = editor, bringIntoViewRequests = bringIntoViewRequests)
      }
    },
    replace = replace@{
        val activeSession = editingSession ?: return@replace
        activeSession.submit { activeEditor, context ->
          activeEditor.scope.launch(context) {
            state.replaceActive(
              editor = activeEditor,
              documentLocked = documentLocked,
              onLocked = { toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.") },
              bringIntoViewRequests = bringIntoViewRequests,
            )
          }
        }
      },
    replaceAll = replaceAll@{
        val activeSession = editingSession ?: return@replaceAll
        activeSession.submit { activeEditor, context ->
          activeEditor.scope.launch(context) {
            state.replaceAllMatches(
              editor = activeEditor,
              documentLocked = documentLocked,
              onLocked = { toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.") },
              bringIntoViewRequests = bringIntoViewRequests,
            )
          }
        }
      },
  )
}

private class FindReplaceSessionController {
  // Serializes suspending session operations that the old runBlocking dispatch kept
  // strictly sequential; mutating flows would otherwise interleave at suspension points.
  private val stateLock = Mutex()
  var active by mutableStateOf(false)
  var findText by mutableStateOf("")
  var replaceText by mutableStateOf("")
  var matches by mutableStateOf(emptyList<FindReplaceMatch>())
  var activeIndex by mutableStateOf<Int?>(null)
  var searchInputFocusRequest by mutableIntStateOf(0)
  private var lastSearchInput by mutableStateOf<FindReplaceSearchInput?>(null)

  suspend fun open(editor: Editor?) {
    stateLock.withLock {
      active = true
      editor?.findReplaceInitialFindTextFromSelection()?.let { findText = it }
      searchInputFocusRequest += 1
      editor?.installFindReplaceDecorations()
    }
  }

  suspend fun close(editor: Editor?) {
    stateLock.withLock {
      active = false
      findText = ""
      replaceText = ""
      clearSearchState(editor)
    }
  }

  fun updateFindText(value: String) {
    findText = value.toSingleLineText()
  }

  fun updateReplaceText(value: String) {
    replaceText = value.toSingleLineText()
  }

  suspend fun runSearch(
    editor: Editor?,
    matchWholeWord: Boolean,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) {
    stateLock.withLock {
      runSearchLocked(
        editor = editor,
        matchWholeWord = matchWholeWord,
        bringIntoViewRequests = bringIntoViewRequests,
      )
    }
  }

  private suspend fun runSearchLocked(
    editor: Editor?,
    matchWholeWord: Boolean,
    bringIntoViewRequests: EditorBringIntoViewRequests,
    force: Boolean = false,
  ) {
    if (!active) return
    val activeEditor = editor ?: return
    val previousInput = lastSearchInput
    val searchInput = FindReplaceSearchInput(findText, matchWholeWord)
    val searchInputChanged = previousInput != searchInput

    if (findText.isEmpty()) {
      if (force || searchInputChanged || matches.isNotEmpty()) {
        lastSearchInput = searchInput
        clearMatches(activeEditor)
      }
      return
    }

    lastSearchInput = searchInput

    if (searchInputChanged) {
      activeIndex = null
    }

    val previousIndex = activeIndex
    val nextMatches =
      activeEditor.findMatches(findText, matchWholeWord).mapIndexed { index, selection ->
        FindReplaceMatch(id = "search-match:$index", selection = selection)
      }
    val nextActiveIndex =
      resolveNextActiveIndex(
        nextMatches = nextMatches,
        previousIndex = previousIndex,
        searchInputChanged = searchInputChanged,
      )

    matches = nextMatches
    activeIndex = nextActiveIndex
    activeEditor.setFindReplaceRanges(
      nextMatches.map { FindReplaceRangeRegistration(id = it.id, selection = it.selection) }
    )
    updateActiveRangeDecoration(activeEditor)
    if (searchInputChanged || force) {
      requestActiveRangeIntoView(activeEditor, bringIntoViewRequests)
    }
  }

  suspend fun findBy(
    offset: Int,
    editor: Editor?,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) {
    stateLock.withLock {
      if (matches.isEmpty()) return
      val currentIndex = activeIndex ?: 0
      activeIndex = (currentIndex + offset + matches.size) % matches.size
      editor?.let {
        updateActiveRangeDecoration(it)
        requestActiveRangeIntoView(it, bringIntoViewRequests)
      }
    }
  }

  suspend fun replaceActive(
    editor: Editor?,
    documentLocked: Boolean,
    onLocked: () -> Unit,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) = stateLock.withLock {
    val activeEditor = editor ?: return@withLock
    if (findText.isEmpty() || replaceText.containsLineBreak()) return@withLock
    if (documentLocked) {
      onLocked()
      return@withLock
    }
    val replaceIndex = activeIndex ?: return@withLock
    val match = matches.getOrNull(replaceIndex) ?: return@withLock

    activeEditor.replaceFindReplaceRangeText(
      id = match.id,
      expectedText = findText,
      replacement = replaceText,
    )
    matches = matches.filterIndexed { index, _ -> index != replaceIndex }
    activeIndex =
      when {
        matches.isEmpty() -> null
        else -> replaceIndex.coerceAtMost(matches.lastIndex)
      }
    runSearchLocked(
      editor = activeEditor,
      matchWholeWord = Preference.searchMatchWholeWord,
      bringIntoViewRequests = bringIntoViewRequests,
      force = true,
    )
  }

  suspend fun replaceAllMatches(
    editor: Editor?,
    documentLocked: Boolean,
    onLocked: () -> Unit,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) = stateLock.withLock {
    val activeEditor = editor ?: return@withLock
    if (findText.isEmpty() || replaceText.containsLineBreak() || matches.isEmpty()) return@withLock
    if (documentLocked) {
      onLocked()
      return@withLock
    }

    activeEditor.replaceAllFindReplaceRanges(
      matches = matches,
      expectedText = findText,
      replacement = replaceText,
    )
    matches = emptyList()
    activeIndex = null
    runSearchLocked(
      editor = activeEditor,
      matchWholeWord = Preference.searchMatchWholeWord,
      bringIntoViewRequests = bringIntoViewRequests,
      force = true,
    )
  }

  private fun activeMatchId(): String? = activeIndex?.let { matches.getOrNull(it)?.id }

  private fun clearMatches(editor: Editor?) {
    val shouldClearRanges =
      matches.isNotEmpty() ||
        editor?.state?.trackedRanges?.searchMatchRanges()?.isNotEmpty() == true
    matches = emptyList()
    activeIndex = null
    if (shouldClearRanges) {
      editor?.clearFindReplaceRanges()
    }
  }

  private fun clearSearchState(editor: Editor?) {
    lastSearchInput = null
    clearMatches(editor)
  }

  private fun resolveNextActiveIndex(
    nextMatches: List<FindReplaceMatch>,
    previousIndex: Int?,
    searchInputChanged: Boolean,
  ): Int? =
    when {
      nextMatches.isEmpty() -> null
      searchInputChanged || previousIndex == null -> 0
      else -> previousIndex.coerceIn(0, nextMatches.lastIndex)
    }

  private suspend fun updateActiveRangeDecoration(editor: Editor) {
    editor.setActiveFindReplaceRange(
      activeId = activeMatchId(),
      currentRanges = editor.state.trackedRanges,
    )
  }

  private fun requestActiveRangeIntoView(
    editor: Editor,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) {
    val target = editor.state.trackedRanges.searchMatchScrollTarget(activeMatchId()) ?: return
    bringIntoViewRequests.requestForVersion(
      target = target,
      version = editor.state.version,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
  }
}

private fun String.toSingleLineText(): String = replace('\r', ' ').replace('\n', ' ')

private fun String.containsLineBreak(): Boolean = any { it == '\r' || it == '\n' }

private fun Editor.findReplaceInitialFindTextFromSelection(): String? {
  if (selection.isCollapsed()) return null
  return copySelection()?.text?.toSingleLineText()
}
