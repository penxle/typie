package co.typie.domain.note

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
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
  fun exitingItemAndSiblingBoundsAnimationDoNotProduceANegativeSize() = runComposeUiTest {
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
            expanded = false,
            isSaving = false,
            saveStatus = NoteSaveStatus.NONE,
            hasPendingColor = false,
            isDirty = false,
            isDeleting = false,
            isChangingStatus = false,
            autoFocusContent = false,
            isEntering = false,
            isExiting = exiting,
            isExitVisible = exiting,
          )
        val siblingItem = item.copy(note = sibling, isExiting = false, isExitVisible = false)
        val items = if (removed) listOf(siblingItem) else listOf(item, siblingItem)
        val reorderState =
          rememberNoteListReorderState(items = items, scrollState = rememberScrollState())

        Box(Modifier.size(width = 400.dp, height = 700.dp)) {
          NoteList(
            identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
            emptyMessage = "",
            queryState = QueryState.Success(Unit),
            items = items,
            authoritativeNotes = listOf(note, sibling),
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
