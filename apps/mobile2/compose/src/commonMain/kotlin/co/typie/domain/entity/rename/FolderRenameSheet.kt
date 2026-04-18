package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private class FolderRenameForm(scope: CoroutineScope, initialName: String) : FormState(scope) {
  val name = field(initialName) { validateOn(ValidateOn.Change) { required("폴더 이름을 입력해주세요.") } }
}

internal interface FolderRenameSheetModel {
  suspend fun renameFolder(
    folderId: String,
    currentName: String,
    name: String,
  ): Result<Unit, Nothing>
}

@Composable
context(_: SheetScope<Unit>)
internal fun FolderRenameSheet(
  model: FolderRenameSheetModel,
  folderId: String,
  initialName: String,
  onUpdated: () -> Unit = {},
) {
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form = remember(folderId, initialName) { FolderRenameForm(scope, initialName) }
  val normalizedInitialName = initialName.trim()
  val trimmedName = form.name.value.trim()
  val canSubmit = trimmedName.isNotEmpty() && trimmedName != normalizedInitialName
  var isSubmitting by remember { mutableStateOf(false) }

  suspend fun submit() {
    if (isSubmitting) return

    val nextName = form.name.value.trim()
    if (nextName.isEmpty()) {
      form.validate()
      return
    }

    if (nextName == normalizedInitialName) {
      dismiss()
      return
    }

    if (!form.validate()) {
      return
    }

    isSubmitting = true
    model
      .renameFolder(folderId = folderId, currentName = normalizedInitialName, name = nextName)
      .withDefaultExceptionHandler(toast)
      .onOk {
        form.name.commit()
        onUpdated()
        dismiss()
      }
    isSubmitting = false
  }

  SheetLayout(
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "취소",
            color = AppTheme.colors.textDefault,
            enabled = !isSubmitting,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = "이름 변경",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        },
        trailing = {
          SheetBarTextButton(
            text = "저장",
            color = AppTheme.colors.textDefault,
            enabled = canSubmit,
            loading = isSubmitting,
            onClick = { submit() },
          )
        },
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
      TextField(
        field = form.name,
        label = "폴더 이름",
        placeholder = "폴더 이름",
        autoFocus = true,
        imeAction = ImeAction.Done,
        onImeAction = { scope.launch { submit() } },
      )
    }
  }
}
