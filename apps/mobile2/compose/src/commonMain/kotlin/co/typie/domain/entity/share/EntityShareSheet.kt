package co.typie.domain.entity

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.DocumentShare_entity
import co.typie.graphql.fragment.FolderShare_entity
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss

@Composable
context(_: SheetScope<Unit>)
internal fun FolderEntityShareSheet(entityIds: List<String>, onUpdated: () -> Unit = {}) {
  val resolvedEntityIds =
    remember(entityIds) { entityIds.map(String::trim).filter(String::isNotEmpty) }
  val model =
    viewModel(key = "folder-entity-share:${resolvedEntityIds.sorted().joinToString(",")}") {
      EntityShareViewModel(
        entityIds = resolvedEntityIds,
        placeholderData = folderEntitySharePlaceholderData(resolvedEntityIds),
      )
    }
  EntityShareErrorEffect(state = model.query.state, onRetry = model::refetch)

  val folders = model.query.data.entities.map { it.folderShare_entity }
  if (!folders.isFolderShareSupported()) {
    EntityShareUnsupportedEffect()
    return
  }

  FolderShareSheet(
    model = model,
    folders = folders,
    loading = model.query.state !is QueryState.Success,
    onUpdated = {
      model.refetch()
      onUpdated()
    },
  )
}

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentEntityShareSheet(entityIds: List<String>, onUpdated: () -> Unit = {}) {
  val resolvedEntityIds =
    remember(entityIds) { entityIds.map(String::trim).filter(String::isNotEmpty) }
  val model =
    viewModel(key = "document-entity-share:${resolvedEntityIds.sorted().joinToString(",")}") {
      EntityShareViewModel(
        entityIds = resolvedEntityIds,
        placeholderData = documentEntitySharePlaceholderData(resolvedEntityIds),
      )
    }
  EntityShareErrorEffect(state = model.query.state, onRetry = model::refetch)

  val documents = model.query.data.entities.map { it.documentShare_entity }
  if (!documents.isDocumentShareSupported()) {
    EntityShareUnsupportedEffect()
    return
  }

  DocumentShareSheet(
    model = model,
    documents = documents,
    loading = model.query.state !is QueryState.Success,
    onUpdated = {
      model.refetch()
      onUpdated()
    },
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun EntityShareErrorEffect(state: QueryState<*>, onRetry: () -> Unit) {
  val dialog = LocalDialog.current
  if (state is QueryState.Error) {
    LaunchedEffect(state.exception) {
      val result =
        dialog.confirm(
          title = "공유 정보를 불러오지 못했어요",
          message = "잠시 후 다시 시도해주세요.",
          confirmText = "다시 시도",
          cancelText = "닫기",
        )
      if (result is DialogResult.Resolved) {
        onRetry()
      } else {
        dismiss()
      }
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun EntityShareUnsupportedEffect() {
  val dialog = LocalDialog.current
  LaunchedEffect(Unit) {
    dialog.alert(title = "공유 정보를 표시할 수 없어요", message = "선택한 항목의 공유 설정을 열 수 없어요.")
    dismiss()
  }
}

private fun List<FolderShare_entity>.isFolderShareSupported(): Boolean {
  return isNotEmpty() && all { it.node.onFolder != null }
}

private fun List<DocumentShare_entity>.isDocumentShareSupported(): Boolean {
  return isNotEmpty() && all { it.node.onDocument != null }
}
