package co.typie.screen.editor.editor.template

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.Editor
import co.typie.icons.Lucide
import co.typie.result.Result
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Spinner
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

private const val LOAD_TEMPLATE_FAILURE_MESSAGE = "템플릿을 불러오는 데 실패했습니다"

@Composable
context(_: SheetScope<Unit>)
internal fun EditorTemplateSheet(editor: Editor) {
  val toast = LocalToast.current
  val model = viewModel { EditorTemplateSheetViewModel() }

  suspend fun insert(template: EditorTemplateSheetTemplate) {
    if (model.insertingTemplateId != null) {
      return
    }

    when (model.insertTemplate(template) { graph -> editor.insertTemplateFragment(graph) }) {
      is Result.Ok -> dismiss()
      else -> toast.show(ToastType.Error, LOAD_TEMPLATE_FAILURE_MESSAGE)
    }
  }

  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "템플릿 불러오기",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
    body = {
      when (val state = model.contentState) {
        EditorTemplateSheetContentState.Loading -> TemplateSheetLoading()
        EditorTemplateSheetContentState.Error -> TemplateSheetError(onRetry = model::refetch)
        EditorTemplateSheetContentState.Empty -> TemplateSheetEmpty()
        is EditorTemplateSheetContentState.Ready ->
          TemplateSheetList(
            templates = state.templates,
            insertingTemplateId = model.insertingTemplateId,
            onTemplateClick = ::insert,
          )
      }
    },
  )
}

@Composable
private fun TemplateSheetLoading() {
  Skeleton(enabled = true) {
    TemplateSheetList(
      templates =
        Skeleton.list(3) { text(8..14) }
          .mapIndexed { index, title ->
            EditorTemplateSheetTemplate(
              id = "placeholder-$index",
              title = title,
              slug = "placeholder-$index",
            )
          },
      insertingTemplateId = null,
      onTemplateClick = {},
    )
  }
}

@Composable
private fun TemplateSheetError(onRetry: suspend () -> Unit) {
  Column(
    modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text(
      text = "템플릿을 불러올 수 없어요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
      textAlign = TextAlign.Center,
    )
    Button(
      text = "다시 시도",
      onClick = onRetry,
      modifier = Modifier.fillMaxWidth(),
      variant = ButtonVariant.Secondary,
    )
  }
}

@Composable
private fun TemplateSheetEmpty() {
  Box(
    modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = "아직 템플릿이 없어요.\n에디터 상단 더보기 메뉴에서\n기존 문서를 템플릿으로 전환해보세요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
      textAlign = TextAlign.Center,
    )
  }
}

@Composable
private fun TemplateSheetList(
  templates: List<EditorTemplateSheetTemplate>,
  insertingTemplateId: String?,
  onTemplateClick: suspend (EditorTemplateSheetTemplate) -> Unit,
) {
  SheetOptionList(items = templates) { template ->
    val inserting = insertingTemplateId == template.id
    SheetOptionRow(
      selected = false,
      enabled = insertingTemplateId == null,
      onClick = { onTemplateClick(template) },
      contentPadding = PaddingValues(horizontal = 16.dp, vertical = 14.dp),
      trailing = {
        if (inserting) {
          Spinner(color = AppTheme.colors.textMuted)
        } else {
          Text(
            text = "사용하기",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textMuted,
          )
        }
      },
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        Icon(
          icon = Lucide.LayoutTemplate,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textMuted,
        )
        Text(
          text = template.title,
          modifier = Modifier.weight(1f),
          style = AppTheme.typography.body,
          color = AppTheme.colors.textDefault,
          overflow = TextOverflow.Ellipsis,
          maxLines = 1,
        )
      }
    }
  }
}
