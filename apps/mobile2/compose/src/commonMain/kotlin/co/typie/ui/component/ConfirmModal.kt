package co.typie.ui.component

import androidx.compose.runtime.Composable
import co.typie.ui.theme.AppTheme

@Composable
fun ConfirmModal(
  title: String,
  message: String,
  confirmText: String = "확인",
  cancelText: String = "취소",
  confirmIsDestructive: Boolean = false,
  onConfirm: suspend () -> Unit,
  onDismiss: suspend () -> Unit,
) {
  BaseModal(
    title = title,
    message = message,
    onDismissRequest = onDismiss,
  ) {
    BaseModalActionButton(
      text = cancelText,
      onClick = onDismiss,
    )
    BaseModalActionDivider()
    BaseModalActionButton(
      text = confirmText,
      color = if (confirmIsDestructive) AppTheme.colors.danger else AppTheme.colors.brand,
      onClick = onConfirm,
    )
  }
}
