package co.typie.ui.component.dialog

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Typie
import co.typie.navigation.Navigator
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

suspend fun Dialog.alert(
  title: String,
  message: String,
  confirmText: String = "확인",
): DialogResult<Unit> =
  present(dismissible = true) {
    DialogLayout(
      title = title,
      message = message,
      actions = { DialogActionButton(text = confirmText) { resolve(Unit) } },
    )
  }

suspend fun Dialog.confirm(
  title: String,
  message: String,
  confirmText: String = "확인",
  cancelText: String = "취소",
  confirmIsDestructive: Boolean = false,
): DialogResult<Unit> =
  present(dismissible = true) {
    DialogLayout(
      title = title,
      message = message,
      actions = {
        DialogActionButton(text = cancelText) { dismiss() }
        DialogActionDivider()
        DialogActionButton(
          text = confirmText,
          color = if (confirmIsDestructive) AppTheme.colors.danger else AppTheme.colors.textDefault,
        ) {
          resolve(Unit)
        }
      },
    )
  }

suspend fun Dialog.error(
  nav: Navigator,
  title: String = "문제가 발생했어요",
  message: String = "잠시 후 다시 시도해주세요.",
  onRetry: () -> Unit,
) {
  val canPop = nav.canPop

  present(dismissible = false) {
    DialogLayout(
      title = title,
      message = message,
      icon = {
        Box(
          modifier =
            Modifier.size(48.dp).background(AppTheme.colors.dangerSubtle, AppShapes.circle),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            Typie.Exclamation,
            modifier = Modifier.size(20.dp),
            tint = AppTheme.colors.textOnDangerSubtle,
          )
        }
      },
      actions = { DialogActionButton(text = if (canPop) "뒤로 가기" else "다시 시도") { resolve(Unit) } },
    )
  }

  if (canPop) nav.pop() else onRetry()
}
