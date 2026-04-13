package co.typie.screen.space.document

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import co.typie.overlay.LocalToast
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.TextField
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.HeaderTextAction
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private class DocumentRenameForm(scope: CoroutineScope, initialTitle: String) : FormState(scope) {
  val title = field(initialTitle) { validateOn(ValidateOn.Change) { required("문서 이름을 입력해주세요.") } }
}

internal interface DocumentRenameSheetModel {
  suspend fun updateDocument(
    documentId: String,
    currentTitle: String,
    title: String,
  ): Result<Unit, Nothing>
}

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentRenameContent(
  model: DocumentRenameSheetModel,
  documentId: String,
  initialTitle: String,
  onUpdated: () -> Unit = {},
) {
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form = remember(documentId, initialTitle) { DocumentRenameForm(scope, initialTitle) }
  val normalizedInitialTitle = initialTitle.trim()
  val trimmedTitle = form.title.value.trim()
  val canSubmit = trimmedTitle.isNotEmpty() && trimmedTitle != normalizedInitialTitle
  var isSubmitting by remember { mutableStateOf(false) }

  suspend fun submit() {
    if (isSubmitting) return

    val nextTitle = form.title.value.trim()
    if (nextTitle.isEmpty()) {
      form.validate()
      return
    }

    if (nextTitle == normalizedInitialTitle) {
      dismiss()
      return
    }

    if (!form.validate()) {
      return
    }

    isSubmitting = true
    model
      .updateDocument(
        documentId = documentId,
        currentTitle = normalizedInitialTitle,
        title = nextTitle,
      )
      .withDefaultExceptionHandler(toast)
      .onOk {
        form.title.commit()
        onUpdated()
        dismiss()
      }
    isSubmitting = false
  }

  SheetLayout(
    header = {
      ActionHeader(
        title = "이름 변경",
        leading = {
          HeaderTextAction(
            text = "취소",
            color = AppTheme.colors.brand,
            enabled = !isSubmitting,
            onClick = { dismiss() },
          )
        },
        trailing = {
          HeaderTextAction(
            text = "저장",
            color = AppTheme.colors.brand,
            textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
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
        field = form.title,
        label = "문서 이름",
        placeholder = "문서 이름",
        autoFocus = true,
        imeAction = ImeAction.Done,
        onImeAction = { scope.launch { submit() } },
      )
    }
  }
}
