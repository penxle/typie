package co.typie.ui.component.dialog

sealed interface DialogResult<out R> {
  data class Resolved<R>(val value: R) : DialogResult<R>

  data object Dismissed : DialogResult<Nothing>
}
