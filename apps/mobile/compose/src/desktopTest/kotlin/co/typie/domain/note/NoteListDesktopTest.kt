package co.typie.domain.note

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.datetime.timeAgo
import co.typie.graphql.QueryState
import co.typie.graphql.type.NoteStatus
import co.typie.result.Result
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class NoteListDesktopTest {
  @Test
  fun collapsedMetadataFitsWithinANarrowCard() = runComposeUiTest {
    val note =
      notesNote(
        id = "narrow",
        entities =
          listOf(
            notesDocumentEntity(id = "1", title = "아주 긴 문서 제목입니다"),
            notesDocumentEntity(id = "2"),
            notesDocumentEntity(id = "3"),
          ),
      )
    val expectedTime = note.updatedAt.timeAgo()

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.width(96.dp).testTag("metadata")) { NoteCollapsedMetaRow(note) }
      }
    }
    waitForIdle()

    val container = onNodeWithTag("metadata").fetchSemanticsNode().boundsInRoot
    listOf("아주 긴 문서 제목입니다", "+2", "·", expectedTime).forEach { text ->
      val bounds = onNodeWithText(text).fetchSemanticsNode().boundsInRoot
      assertTrue(bounds.left >= container.left, "$text starts outside $container: $bounds")
      assertTrue(bounds.right <= container.right, "$text ends outside $container: $bounds")
    }
  }

  @Test
  fun activeNoteColorChangeDoesNotRecomposeInactivePresentation() = runComposeUiTest {
    val activeNote = notesNote(id = "active", color = "gray")
    val inactiveNote = notesNote(id = "inactive", color = "gray")
    lateinit var editState: NoteEditState
    var activeCompositions = 0
    var inactiveCompositions = 0

    setContent {
      val scope = rememberCoroutineScope()
      editState = remember(scope) { NoteEditState(scope = scope).also { it.open(activeNote) } }

      WithNoteEditPresentation(note = activeNote, editState = editState) { presentation ->
        Box(Modifier.size(if (presentation.note.color == "red") 2.dp else 1.dp))
        SideEffect { activeCompositions += 1 }
      }
      WithNoteEditPresentation(note = inactiveNote, editState = editState) { presentation ->
        Box(Modifier.size(if (presentation.note.color == "red") 2.dp else 1.dp))
        SideEffect { inactiveCompositions += 1 }
      }
    }
    waitForIdle()
    val activeBefore = activeCompositions
    val inactiveBefore = inactiveCompositions

    runOnUiThread {
      editState.updateColor(siteId = "site", noteId = activeNote.id, value = "red") { _, _ ->
        NoteSaveOutcome.Saved
      }
    }
    waitForIdle()

    assertTrue(activeCompositions > activeBefore)
    assertTrue(
      inactiveCompositions == inactiveBefore,
      "Inactive presentation recomposed: $inactiveBefore -> $inactiveCompositions",
    )
  }

  @Test
  fun exitingItemAndSiblingPlacementAnimationDoNotProduceANegativeSize() = runComposeUiTest {
    val note = notesNote(id = "exiting", content = "content")
    val sibling = notesNote(id = "sibling", content = "sibling", order = "200")
    var exiting by mutableStateOf(false)
    var removed by mutableStateOf(false)

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalHazeBlurStyle provides
          HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
        LocalToast provides Toast(),
      ) {
        val item =
          NoteListItem(
            note = note,
            isDeleting = false,
            isChangingStatus = false,
            isEntering = false,
            isExiting = exiting,
            isExitVisible = exiting,
          )
        val siblingItem = item.copy(note = sibling, isExiting = false, isExitVisible = false)
        val items = if (removed) listOf(siblingItem) else listOf(item, siblingItem)
        val scope = rememberCoroutineScope()
        val editState = remember(scope) { NoteEditState(scope) }
        val lazyListState = rememberLazyListState()
        val reorderState = rememberNoteListReorderState(items = items, scrollState = lazyListState)

        Box(Modifier.size(width = 400.dp, height = 700.dp)) {
          NoteList(
            identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
            emptyMessage = "",
            queryState = QueryState.Success(Unit),
            items = items,
            authoritativeNotes = listOf(note, sibling),
            editState = editState,
            onEnterAnimationFinished = {},
            onExitAnimationFinished = { removed = true },
            reorderState = reorderState,
            noteColorOptions =
              listOf(NoteColorOption(value = "gray", label = "그레이", stroke = Color.Gray)),
            interactive = true,
            onRetry = {},
            actions =
              NoteListActions(
                onExpand = {},
                onCollapse = {},
                onCreateNote = {},
                onContentChange = { _, _ -> },
                onBlur = {},
                onToggleStatus = {},
                onColorChange = { _, _ -> },
                onAddEntity = {},
                onEntityClick = { _, _ -> },
                onDelete = {},
                onMoveNote = { _, _, _ -> Result.Ok("") },
              ),
          )
        }
      }
    }
    waitForIdle()

    mainClock.autoAdvance = false
    runOnUiThread { exiting = true }
    val siblingPositions = mutableListOf<Float>()
    repeat(70) {
      mainClock.advanceTimeByFrame()
      siblingPositions += onNodeWithText("sibling").fetchSemanticsNode().boundsInRoot.top
    }

    val largestDownwardStep =
      siblingPositions.zipWithNext().maxOfOrNull { (previous, next) -> next - previous } ?: 0f
    assertTrue(
      largestDownwardStep < 2f,
      "Sibling moved down by $largestDownwardStep px after the exiting item was removed",
    )
  }
}
