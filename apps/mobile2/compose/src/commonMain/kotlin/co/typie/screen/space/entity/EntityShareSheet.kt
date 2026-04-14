package co.typie.screen.space.entity

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
import co.typie.screen.space.document.DocumentShareContent
import co.typie.screen.space.document.DocumentShareTarget
import co.typie.screen.space.folder.FolderShareContent
import co.typie.screen.space.folder.FolderShareTarget
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
internal fun EntityShareContent(entityIds: List<String>, onUpdated: () -> Unit = {}) {
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
      val entities = state.data.entities
      val kind = resolveEntityShareKind(entities.map { it.type })

      when (kind) {
        EntityShareKind.Folder -> {
          val folders = entities.mapNotNull { entity ->
            val folder = entity.node.onFolder ?: return@mapNotNull null
            FolderShareTarget(
              id = folder.id,
              url = entity.url,
              visibility = entity.visibility,
              thumbnailId = folder.thumbnail?.id,
              thumbnailUrl = folder.thumbnail?.url,
            )
          }
          if (folders.isEmpty() || folders.size != entities.size) {
            EntityShareStatusContent(message = "공유 정보를 표시할 수 없어요.")
            return
          }

          FolderShareContent(
            model = model,
            folders = folders,
            onUpdated = {
              model.refetch()
              onUpdated()
            },
          )
        }

        EntityShareKind.Document -> {
          val documents = entities.mapNotNull { entity ->
            val document = entity.node.onDocument ?: return@mapNotNull null
            DocumentShareTarget(
              id = document.id,
              url = entity.url,
              visibility = entity.visibility,
              contentRating = document.contentRating,
              password = document.password,
              allowReaction = document.allowReaction,
              protectContent = document.protectContent,
              thumbnailId = document.thumbnail?.id,
              thumbnailUrl = document.thumbnail?.url,
            )
          }
          if (documents.isEmpty() || documents.size != entities.size) {
            EntityShareStatusContent(message = "공유 정보를 표시할 수 없어요.")
            return
          }

          DocumentShareContent(
            model = model,
            documents = documents,
            onUpdated = {
              model.refetch()
              onUpdated()
            },
          )
        }

        null -> {
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
