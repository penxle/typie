package co.typie.ui.component.editorsettings

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.LayoutMode
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.graphql.type.FontFamilySource
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.component.sheet.SheetOverlay
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class EditorSettingsControlsDesktopTest {
  @Test
  fun font_family_sheet_reveals_selected_family() = runComposeUiTest {
    val sheet = Sheet()
    val families = (0 until 8).map(::fontFamily)

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalHazeBlurStyle provides
          HazeBlurStyle {
            blurRadius(20.dp)
            noiseFactor(0f)
            colorEffects(emptyList())
          },
      ) {
        Box(Modifier.size(width = 400.dp, height = 280.dp)) {
          EditorSettingsFontSection(
            style = EditorStyleSettings(fontFamily = "Family 7"),
            fontFamilies = families,
            sheet = sheet,
            onStyleChange = {},
          )
          SheetOverlay(sheet)
        }
      }
    }

    onNodeWithText("패밀리").performClick()
    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()

    onNode(hasText("Family 7") and hasAnyAncestor(hasScrollAction()), useUnmergedTree = true)
      .assertIsDisplayed()
  }

  @Test
  fun namedPageSizesOfferMarginChips() = runComposeUiTest {
    setLayoutSection(LayoutMode.Paginated(794, 1123, 94, 94, 94, 94))

    onNodeWithText("좁게").assertExists()
    onNodeWithText("보통").assertExists()
    onNodeWithText("넓게").assertExists()
  }

  @Test
  fun customPageSizesOfferNoMarginChips() = runComposeUiTest {
    setLayoutSection(LayoutMode.Paginated(756, 1058, 94, 94, 94, 94))

    onAllNodesWithText("좁게").assertCountEquals(0)
    onAllNodesWithText("보통").assertCountEquals(0)
    onAllNodesWithText("넓게").assertCountEquals(0)
    onAllNodesWithText("사용자 정의").assertCountEquals(2)
  }

  private fun ComposeUiTest.setLayoutSection(layout: LayoutMode) {
    val sheet = Sheet()

    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalHazeBlurStyle provides
          HazeBlurStyle {
            blurRadius(20.dp)
            noiseFactor(0f)
            colorEffects(emptyList())
          },
      ) {
        Box(Modifier.size(width = 400.dp, height = 600.dp)) {
          EditorSettingsLayoutSection(layout = layout, sheet = sheet, onLayoutChange = {})
          SheetOverlay(sheet)
        }
      }
    }
  }

  private fun fontFamily(index: Int) =
    EditorSettingsFontFamily_family(
      __typename = "DocumentFontFamily",
      id = "family-$index",
      familyName = "Family $index",
      displayName = "Family $index",
      source = FontFamilySource.DEFAULT,
      state = FontFamilyState.ACTIVE,
      fonts =
        listOf(
          EditorSettingsFontFamily_family.Font(
            __typename = "DocumentFont",
            id = "font-$index",
            weight = 400,
            state = FontState.ACTIVE,
            subfamilyDisplayName = null,
          )
        ),
    )
}
