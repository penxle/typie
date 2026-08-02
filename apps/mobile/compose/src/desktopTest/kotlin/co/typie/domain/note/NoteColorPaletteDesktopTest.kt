package co.typie.domain.note

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class NoteColorPaletteDesktopTest {
  @Test
  fun verticalDragBeforeArmDelayScrollsParentWithoutChangingColor() = runComposeUiTest {
    val colors = mutableListOf<String>()
    var scrollValue = 0

    setContent {
      val scrollState = rememberScrollState()
      scrollValue = scrollState.value
      Column(
        Modifier.size(width = PaletteWidth, height = PaletteHeight).verticalScroll(scrollState)
      ) {
        TestPalette(onColorChange = colors::add)
        Spacer(Modifier.height(200.dp))
      }
    }
    waitForIdle()

    onNodeWithTag(PaletteTag).performTouchInput {
      down(center)
      moveBy(Offset(x = 0f, y = -100f), delayMillis = 16L)
      up()
    }
    waitForIdle()

    assertTrue(scrollValue > 0)
    assertTrue(colors.isEmpty())
  }

  @Test
  fun horizontalDragBeforeArmDelayScrollsParentWithoutChangingColor() = runComposeUiTest {
    val colors = mutableListOf<String>()
    var scrollValue = 0

    setContent {
      val scrollState = rememberScrollState()
      scrollValue = scrollState.value
      Row(
        Modifier.size(width = PaletteWidth, height = PaletteHeight).horizontalScroll(scrollState)
      ) {
        TestPalette(onColorChange = colors::add)
        Spacer(Modifier.width(200.dp))
      }
    }
    waitForIdle()

    onNodeWithTag(PaletteTag).performTouchInput {
      down(center)
      moveBy(Offset(x = -100f, y = 0f), delayMillis = 16L)
      up()
    }
    waitForIdle()

    assertTrue(scrollValue > 0)
    assertTrue(colors.isEmpty())
  }

  @Test
  fun holdThenHorizontalDragScrubsToPointerColor() = runComposeUiTest {
    val colors = mutableListOf<String>()

    setContent { TestPalette(onColorChange = colors::add) }
    waitForIdle()

    onNodeWithTag(PaletteTag).performTouchInput {
      down(Offset(x = width / 6f, y = center.y))
      advanceEventTime(NoteColorPaletteScrubArmDelayMillis)
      moveTo(Offset(x = width * 5f / 6f, y = center.y), delayMillis = 16L)
      up()
    }
    waitForIdle()

    assertContentEquals(listOf("blue"), colors)
  }

  @Test
  fun holdThenVerticalDragScrollsParentWithoutChangingColor() = runComposeUiTest {
    val colors = mutableListOf<String>()
    var scrollValue = 0

    setContent {
      val scrollState = rememberScrollState()
      scrollValue = scrollState.value
      Column(
        Modifier.size(width = PaletteWidth, height = PaletteHeight).verticalScroll(scrollState)
      ) {
        TestPalette(onColorChange = colors::add)
        Spacer(Modifier.height(200.dp))
      }
    }
    waitForIdle()

    onNodeWithTag(PaletteTag).performTouchInput {
      down(center)
      advanceEventTime(NoteColorPaletteScrubArmDelayMillis)
      moveBy(Offset(x = 0f, y = -100f), delayMillis = 16L)
      up()
    }
    waitForIdle()

    assertTrue(scrollValue > 0)
    assertTrue(colors.isEmpty())
  }

  @Test
  fun tapSelectsColorExactlyOnce() = runComposeUiTest {
    val colors = mutableListOf<String>()

    setContent { TestPalette(onColorChange = colors::add) }
    waitForIdle()

    onNodeWithTag(PaletteTag).performTouchInput { click(Offset(x = width / 2f, y = center.y)) }
    waitForIdle()

    assertContentEquals(listOf("red"), colors)
  }

  @Composable
  private fun TestPalette(onColorChange: (String) -> Unit) {
    val options = remember {
      listOf(
        NoteColorOption(value = "gray", label = "그레이", stroke = Color.Gray),
        NoteColorOption(value = "red", label = "레드", stroke = Color.Red),
        NoteColorOption(value = "blue", label = "블루", stroke = Color.Blue),
      )
    }
    NoteColorPalette(
      noteColorOptions = options,
      selectedColor = "gray",
      onColorChange = onColorChange,
      modifier = Modifier.testTag(PaletteTag),
    )
  }

  private companion object {
    const val PaletteTag = "note-color-palette"
    val PaletteWidth = 60.dp
    val PaletteHeight = 24.dp
  }
}
