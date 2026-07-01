package co.typie.screen.editor.editor.findreplace

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType

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
  editor: Editor?,
  editorState: EditorState,
  bringIntoViewRequests: EditorBringIntoViewRequests,
): EditorFindReplaceSession {
  val state = remember { FindReplaceSessionController() }
  val toast = LocalToast.current

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
    state.matchWholeWord,
  ) {
    state.runSearch(editor = editor, bringIntoViewRequests = bringIntoViewRequests)
  }

  return EditorFindReplaceSession(
    active = state.active,
    findText = state.findText,
    replaceText = state.replaceText,
    matchWholeWord = state.matchWholeWord,
    matchCount = state.matches.size,
    activeMatchNumber = state.activeIndex?.let { it + 1 },
    searchInputFocusRequest = state.searchInputFocusRequest,
    onOpen = { state.open(editor) },
    close = { state.close(editor) },
    updateFindText = state::updateFindText,
    updateReplaceText = state::updateReplaceText,
    updateMatchWholeWord = { enabled -> state.matchWholeWord = enabled },
    findPrevious = {
      state.findBy(offset = -1, editor = editor, bringIntoViewRequests = bringIntoViewRequests)
    },
    findNext = {
      state.findBy(offset = 1, editor = editor, bringIntoViewRequests = bringIntoViewRequests)
    },
    replace = {
      state.replaceActive(
        editor = editor,
        documentLocked = documentLocked,
        onLocked = { toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.") },
        bringIntoViewRequests = bringIntoViewRequests,
      )
    },
    replaceAll = {
      state.replaceAllMatches(
        editor = editor,
        documentLocked = documentLocked,
        onLocked = { toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.") },
        bringIntoViewRequests = bringIntoViewRequests,
      )
    },
  )
}

private class FindReplaceSessionController {
  var active by mutableStateOf(false)
  var findText by mutableStateOf("")
  var replaceText by mutableStateOf("")
  var matchWholeWord by mutableStateOf(false)
  var matches by mutableStateOf(emptyList<FindReplaceMatch>())
  var activeIndex by mutableStateOf<Int?>(null)
  var searchInputFocusRequest by mutableIntStateOf(0)
  private var lastSearchInput by mutableStateOf<FindReplaceSearchInput?>(null)

  fun open(editor: Editor?) {
    active = true
    editor?.findReplaceInitialFindTextFromSelection()?.let { findText = it }
    searchInputFocusRequest += 1
    editor?.installFindReplaceDecorations()
  }

  fun close(editor: Editor?) {
    active = false
    findText = ""
    replaceText = ""
    matchWholeWord = false
    clearSearchState(editor)
  }

  fun updateFindText(value: String) {
    findText = value.toSingleLineText()
  }

  fun updateReplaceText(value: String) {
    replaceText = value.toSingleLineText()
  }

  fun runSearch(
    editor: Editor?,
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

  fun findBy(offset: Int, editor: Editor?, bringIntoViewRequests: EditorBringIntoViewRequests) {
    if (matches.isEmpty()) return
    val currentIndex = activeIndex ?: 0
    activeIndex = (currentIndex + offset + matches.size) % matches.size
    editor?.let {
      updateActiveRangeDecoration(it)
      requestActiveRangeIntoView(it, bringIntoViewRequests)
    }
  }

  fun replaceActive(
    editor: Editor?,
    documentLocked: Boolean,
    onLocked: () -> Unit,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) {
    val activeEditor = editor ?: return
    if (findText.isEmpty() || replaceText.containsLineBreak()) return
    if (documentLocked) {
      onLocked()
      return
    }
    val replaceIndex = activeIndex ?: return
    val match = matches.getOrNull(replaceIndex) ?: return

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
    runSearch(editor = activeEditor, bringIntoViewRequests = bringIntoViewRequests, force = true)
  }

  fun replaceAllMatches(
    editor: Editor?,
    documentLocked: Boolean,
    onLocked: () -> Unit,
    bringIntoViewRequests: EditorBringIntoViewRequests,
  ) {
    val activeEditor = editor ?: return
    if (findText.isEmpty() || replaceText.containsLineBreak() || matches.isEmpty()) return
    if (documentLocked) {
      onLocked()
      return
    }

    activeEditor.replaceAllFindReplaceRanges(
      matches = matches,
      expectedText = findText,
      replacement = replaceText,
    )
    matches = emptyList()
    activeIndex = null
    runSearch(editor = activeEditor, bringIntoViewRequests = bringIntoViewRequests, force = true)
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

  private fun updateActiveRangeDecoration(editor: Editor) {
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
  return copySelection()?.text?.toSingleLineText()?.takeIf { it.isNotBlank() }
}
