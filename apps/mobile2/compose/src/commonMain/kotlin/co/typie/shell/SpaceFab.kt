package co.typie.shell

import androidx.compose.ui.graphics.Color
import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

data class FabMenuItem(
  val icon: IconData,
  val label: String,
  val tint: Color? = null,
  val onClick: () -> Unit = {},
)

data class FabConfig(
  val icon: IconData = Lucide.SquarePen,
  val onClick: () -> Unit = {},
  val menuItems: List<FabMenuItem> = emptyList(),
)

fun spaceFabConfig(): FabConfig {
  return FabConfig(
    icon = Lucide.SquarePlus,
    menuItems = listOf(
      FabMenuItem(
        icon = Lucide.FolderPlus,
        label = "여기에 폴더 만들기",
      ),
      FabMenuItem(
        icon = Lucide.SquarePen,
        label = "여기에 문서 만들기",
      ),
    ),
  )
}
