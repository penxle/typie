package co.typie.screen.settings.textreplacements

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.colorspace.ColorSpaces.match
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.domain.settings.SettingSwitch
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
context(_: SheetScope<Unit>)
internal fun TextReplacementEditSheet(
  model: TextReplacementsViewModel,
  editing: CustomTextReplacement?,
) {
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val form = remember(editing?.id) { TextReplacementForm(scope, editing) }
  var isSaving by remember { mutableStateOf(false) }
  var isDeleting by remember { mutableStateOf(false) }

  suspend fun submit() {
    if (!form.validate()) return
    isSaving = true

    if (editing == null) {
      model
        .createTextReplacement(
          match = form.match.value,
          substitute = form.substitute.value,
          regex = form.regex.value,
          note = form.note.value,
        )
        .withDefaultExceptionHandler(toast)
        .onOk { complete(Unit) }
    } else {
      model
        .updateTextReplacement(
          id = editing.id,
          match = form.match.value,
          substitute = form.substitute.value,
          regex = form.regex.value,
          note = form.note.value,
        )
        .withDefaultExceptionHandler(toast)
        .onOk { complete(Unit) }
    }

    isSaving = false
  }

  SheetLayout(
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "취소",
            color = AppTheme.colors.textDefault,
            enabled = !isSaving && !isDeleting,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = if (editing != null) "대치 규칙 수정" else "대치 규칙 추가",
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
            enabled = !isDeleting,
            loading = isSaving,
            onClick = { submit() },
          )
        },
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
      Column {
        TextField(
          field = form.match,
          label = "찾을 텍스트",
          labelPosition = LabelPosition.Internal,
          placeholder = "찾을 텍스트를 입력해 주세요",
        )
        TextField(
          field = form.substitute,
          label = "삽입할 텍스트",
          labelPosition = LabelPosition.Internal,
          placeholder = "삽입할 텍스트를 입력해 주세요",
        )
        TextField(
          field = form.note,
          label = "설명 (선택)",
          labelPosition = LabelPosition.Internal,
          placeholder = "설명 (선택)",
        )

        RegexToggleRow(
          checked = form.regex.value,
          onClick = { form.regex.setValue(!form.regex.value) },
          onCheckedChange = { next -> form.regex.setValue(next) },
        )
      }

      if (editing != null) {
        DeleteAction(
          isDeleting = isDeleting,
          enabled = !isSaving && !isDeleting,
          onDelete = {
            val result =
              dialog.confirm(
                title = "대치 규칙 삭제",
                message = "\"${editing.match} → ${editing.substitute}\" 규칙을 삭제하시겠어요?",
                confirmText = "삭제",
                confirmIsDestructive = true,
              )
            if (result is DialogResult.Resolved) {
              isDeleting = true
              model.deleteTextReplacement(editing.id).withDefaultExceptionHandler(toast).onOk {
                complete(Unit)
              }
              isDeleting = false
            }
          },
        )
      }
    }
  }
}

@Composable
private fun RegexToggleRow(
  checked: Boolean,
  onClick: suspend () -> Unit,
  onCheckedChange: (Boolean) -> Unit,
) {
  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .clip(AppShapes.rounded(AppShapes.md))
          .clickable(onClick)
          .pressScale()
          .padding(horizontal = 16.dp, vertical = 4.dp),
      horizontalArrangement = Arrangement.spacedBy(12.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(icon = Lucide.Regex, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textMuted)
      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text("정규식", style = AppTheme.typography.label)
        Text(
          "찾을 텍스트를 정규식 패턴으로 해석합니다.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
        )
      }
      SettingSwitch(checked = checked, onCheckedChange = onCheckedChange)
    }
  }
}

@Composable
private fun DeleteAction(isDeleting: Boolean, enabled: Boolean, onDelete: suspend () -> Unit) {
  val text = if (isDeleting) "삭제 중..." else "이 규칙 삭제하기"
  val alpha by animateFloatAsState(if (enabled) 1f else 0.4f)

  InteractionScope {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .clickable(enabled = enabled, onClick = onDelete)
          .pressScale(0.97f)
          .padding(horizontal = 4.dp, vertical = 8.dp)
          .graphicsLayer { this.alpha = alpha },
      contentAlignment = Alignment.Center,
    ) {
      Text(text = text, style = AppTheme.typography.action, color = AppTheme.colors.danger)
    }
  }
}
