package co.typie.screen.document.bodysettings

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.Editor
import co.typie.editor.EditorLocalChangesetBus
import co.typie.editor.EditorLocalChangesetTracker
import co.typie.editor.EditorScope
import co.typie.editor.FontLoader
import co.typie.graphql.Apollo
import co.typie.graphql.DocumentBodySettingsScreen_PushDocumentChangesets_Mutation
import co.typie.graphql.DocumentBodySettingsScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.executeMutation
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.graphql.type.PushDocumentChangesetsInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.storage.Preference
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal class DocumentBodySettingsViewModel(private val entityId: String) : ViewModel() {
  private val saveMutex = Mutex()
  private val changesetTracker = EditorLocalChangesetTracker()

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = { data ->
        val document = data.entity.node.onDocument ?: return@watchQuery
        FontLoader.loadFonts(
          document.bodySettingsFontFamilies
            .filter { it.editorSettingsFontFamily_family.state == FontFamilyState.ACTIVE }
            .map { family ->
              val activeFontIds =
                family.editorSettingsFontFamily_family.fonts
                  .filter { it.state == FontState.ACTIVE }
                  .map { it.id }
                  .toSet()
              family.fontLoader_FontFamily.copy(
                fonts = family.fontLoader_FontFamily.fonts.filter { it.id in activeFontIds }
              )
            }
        )
      },
    ) {
      DocumentBodySettingsScreen_Query(entityId = entityId)
    }

  val fontFamilies by derivedStateOf {
    query.data.entity.node.onDocument?.bodySettingsFontFamilies.orEmpty().map { family ->
      family.editorSettingsFontFamily_family
    }
  }

  internal suspend fun updateBodySettings(
    editor: Editor,
    documentId: String,
    block: EditorScope.() -> Unit,
  ): Result<ByteArray?, Nothing> = result {
    saveMutex.withLock {
      val changesets = changesetTracker.collect(editor = editor, block = block)
      if (changesets.isEmpty()) return@withLock null

      val response =
        Apollo.executeMutation(
          DocumentBodySettingsScreen_PushDocumentChangesets_Mutation(
            input =
              PushDocumentChangesetsInput(
                changesets = changesets,
                clientId = Preference.deviceId,
                documentId = documentId,
              )
          )
        )
      changesetTracker.markSynced(response.pushDocumentChangesets.heads)
      EditorLocalChangesetBus.publish(entityId = entityId, changesets = changesets)
      changesets
    }
  }
}

private fun placeholderData() =
  DocumentBodySettingsScreen_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-body-settings-entity"
      type = EntityType.DOCUMENT
      node = buildDocument { id = "placeholder-body-settings-document" }
    }
  }
