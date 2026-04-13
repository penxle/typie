package co.typie.ui.component.sheet

import androidx.compose.ui.unit.Dp

sealed interface SheetStop {
  data class Bottom(val height: Dp) : SheetStop

  data class Top(val margin: Dp) : SheetStop
}
