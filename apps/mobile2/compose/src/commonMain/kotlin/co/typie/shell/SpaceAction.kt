package co.typie.shell

import androidx.compose.runtime.Composable
import co.typie.icons.Lucide
import co.typie.ui.component.bottombar.ActionMenuItem
import co.typie.ui.component.bottombar.BottomBarActionButton

@Composable
fun SpaceBottomBarActionButton() {
  BottomBarActionButton(
    icon = Lucide.SquarePlus,
    menus =
      listOf(
        ActionMenuItem(icon = Lucide.FolderPlus, label = "여기에 폴더 만들기"),
        ActionMenuItem(icon = Lucide.SquarePen, label = "여기에 문서 만들기"),
      ),
  )
}
