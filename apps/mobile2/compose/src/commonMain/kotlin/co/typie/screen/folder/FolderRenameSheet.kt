package co.typie.screen.folder

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import co.typie.form.FormState
import co.typie.form.ValidateOn
import co.typie.form.required
import co.typie.ui.component.TextField
import co.typie.ui.component.bottomsheet.BottomSheetHeaderTextAction
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private class FolderRenameForm(
  scope: CoroutineScope,
  initialName: String,
) : FormState(scope) {
  val name = field(initialName) {
    validateOn(ValidateOn.Change) {
      required("폴더 이름을 입력해주세요.")
    }
  }
}

@Composable
fun BottomSheetScope<Unit>.FolderRenameSheet(
  model: FolderViewModel,
  folderId: String,
  initialName: String,
) {
  val scope = rememberCoroutineScope()
  val form = remember(folderId, initialName) {
    FolderRenameForm(scope, initialName)
  }
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
    try {
      model.renameFolder(
        folderId = folderId,
        currentName = normalizedInitialName,
        name = nextName,
        onRenamed = { dismiss() },
      )
    } finally {
      isSubmitting = false
    }
  }

  BottomSheetScaffold(
    title = "이름 변경",
    leadingAction = {
      BottomSheetHeaderTextAction(
        text = "취소",
        color = AppTheme.colors.brand,
        enabled = !isSubmitting,
        onClick = { dismiss() },
      )
    },
    trailingAction = {
      BottomSheetHeaderTextAction(
        text = "저장",
        color = AppTheme.colors.brand,
        textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
        enabled = canSubmit,
        loading = isSubmitting,
        onClick = { submit() },
      )
    },
  ) {
    Column(
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
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
