package co.typie.screen.space.folder

import androidx.compose.ui.unit.dp
import co.typie.ui.component.sheet.SheetDetentContext
import co.typie.ui.component.sheet.SheetDetentId
import co.typie.ui.component.sheet.SheetDetentResolver
import co.typie.ui.component.sheet.SheetSizePolicy
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class FolderIconPickerSheetSpecTest {

  @Test
  fun `icon picker sheet starts collapsed and expands to larger top gap detent`() {
    val spec = folderIconPickerSheetSpec()
    val sizePolicy = assertIs<SheetSizePolicy.Detents>(spec.sizePolicy)

    assertEquals(SheetDetentId.Fixed(360.dp), sizePolicy.initial.id)
    assertEquals(
      listOf(SheetDetentId.Fixed(360.dp), SheetDetentId.TopGap(128.dp)),
      sizePolicy.available.map { it.id },
    )

    val resolved =
      SheetDetentResolver.resolve(
        policy = sizePolicy,
        context = SheetDetentContext(viewportHeight = 800.dp, contentHeight = 600.dp),
      )

    assertEquals(listOf(360.dp, 672.dp), resolved.map { it.height })
  }
}
