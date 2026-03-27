package co.typie.ui.component

import androidx.compose.runtime.Composable

@Composable
fun AlertModal(
  title: String,
  message: String,
  confirmText: String = "확인",
  onConfirm: suspend () -> Unit,
  onDismiss: (suspend () -> Unit)? = null,
) {
  val dismissAction = onDismiss ?: onConfirm

  BaseModal(
    title = title,
    message = message,
    onDismissRequest = dismissAction,
  ) {
    BaseModalActionButton(
      text = confirmText,
      onClick = onConfirm,
    )
  }
}
