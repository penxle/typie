package co.typie.editor

import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.font.FontWeight
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogActionButton
import co.typie.ui.component.dialog.DialogActionDivider
import co.typie.ui.component.dialog.DialogLayout
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.dismiss
import co.typie.ui.component.dialog.resolve
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay

internal suspend fun Dialog.confirmDocumentSave(
  saveState: State<DocumentSaveState>,
  reload: Boolean = false,
): DialogResult<Unit> =
  present(dismissible = !reload) {
    val saved = saveState.value == DocumentSaveState.Protected
    val pending = saveState.value == DocumentSaveState.Pending
    var countdown by remember(saveState) { mutableIntStateOf(5) }
    // Start only when the completed dialog is actually composed, not while it is queued.
    LaunchedEffect(saveState, saved) {
      if (saved) {
        for (remaining in 5 downTo 1) {
          countdown = remaining
          delay(1_000)
        }
        resolve(Unit)
      }
    }

    DialogLayout(
      title =
        when {
          saved -> "저장을 완료했어요"
          reload && pending -> "아직 최신 버전을 불러올 수 없어요"
          reload -> "최신 버전을 불러올 수 없어요"
          pending -> "아직 저장을 완료하지 못했어요"
          else -> "저장하지 못했어요"
        },
      message =
        if (saved) "최근 변경사항을 안전하게 저장했어요."
        else {
          val status = if (pending) "저장이 평소보다 오래 걸리고 있어요." else "안전하게 저장하지 못했어요."
          val action = if (reload) "불러오면" else "닫으면"
          "$status\n지금 $action 최근 변경사항을 잃을 수 있어요."
        },
      actions = {
        if (!reload || !saved) {
          DialogActionButton(text = if (reload) "다시 시도" else "계속 편집") { dismiss() }
          DialogActionDivider()
        }
        DialogActionButton(
          text =
            when {
              saved -> if (reload) "불러오기" else "닫기"
              reload -> "변경사항 버리고 불러오기"
              else -> "저장하지 않고 닫기"
            },
          color = if (saved) AppTheme.colors.textDefault else AppTheme.colors.danger,
          trailing =
            if (saved) {
              {
                Text(
                  text = countdown.toString(),
                  style = AppTheme.typography.action.copy(fontWeight = FontWeight.Bold),
                  color = AppTheme.colors.textMuted,
                )
              }
            } else null,
        ) {
          resolve(Unit)
        }
      },
    )
  }
