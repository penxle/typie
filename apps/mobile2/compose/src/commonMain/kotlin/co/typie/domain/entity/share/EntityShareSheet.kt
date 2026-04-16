package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.QueryState
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.theme.AppTheme

private val EntityShareHeaderPadding = 24.dp

@Composable
context(_: SheetScope<Unit>)
internal fun EntityShareSheet(entityIds: List<String>, onUpdated: () -> Unit = {}) {
  val resolvedEntityIds =
    remember(entityIds) { entityIds.map(String::trim).filter(String::isNotEmpty) }
  val model =
    viewModel(key = "entity-share:${resolvedEntityIds.sorted().joinToString(",")}") {
      EntityShareViewModel(resolvedEntityIds)
    }
  val state = model.query.state

  when (state) {
    QueryState.Loading -> {
      EntityShareStatusContent(message = "공유 정보를 불러오는 중이에요.")
    }

    is QueryState.Error -> {
      EntityShareStatusContent(message = "공유 정보를 불러오지 못했어요.")
    }

    is QueryState.Success -> {
      val handleUpdated = {
        model.refetch()
        onUpdated()
      }
      val entities = state.data.entities
      val folders =
        entities
          .filter { entity -> entity.folderShare_entity.node.onFolder != null }
          .map { entity -> entity.folderShare_entity }
      val documents =
        entities
          .filter { entity -> entity.documentShare_entity.node.onDocument != null }
          .map { entity -> entity.documentShare_entity }

      when {
        folders.isNotEmpty() && folders.size == entities.size -> {
          FolderShareSheet(model = model, folders = folders, onUpdated = handleUpdated)
        }

        documents.isNotEmpty() && documents.size == entities.size -> {
          DocumentShareSheet(model = model, documents = documents, onUpdated = handleUpdated)
        }

        else -> {
          EntityShareStatusContent(message = "공유 정보를 표시할 수 없어요.")
        }
      }
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun EntityShareStatusContent(message: String) {
  SheetLayout(
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        SheetBar(
          leading = {
            SheetBarTextButton(text = "완료", color = AppTheme.colors.brand, onClick = { dismiss() })
          },
          center = {
            Text(
              text = "공유하기",
              style = AppTheme.typography.title,
              color = AppTheme.colors.textPrimary,
              overflow = TextOverflow.Ellipsis,
              maxLines = 1,
            )
          },
        )

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Text(
      text = message,
      modifier = Modifier.fillMaxWidth().padding(horizontal = EntityShareHeaderPadding),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )
  }
}
