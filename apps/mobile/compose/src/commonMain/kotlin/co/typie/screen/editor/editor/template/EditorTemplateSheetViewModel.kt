package co.typie.screen.editor.editor.template

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.EditorTemplateSheet_Query
import co.typie.graphql.EditorTemplateSheet_TemplateGraph_Query
import co.typie.graphql.QueryState
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import co.typie.storage.Preference
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy

internal class EditorTemplateSheetViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, skip = { Preference.siteId == null }) {
      EditorTemplateSheet_Query(siteId = Preference.siteId!!)
    }

  var insertingTemplateId: String? by mutableStateOf(null)
    private set

  val contentState: EditorTemplateSheetContentState by derivedStateOf {
    resolveEditorTemplateSheetContentState(siteId = Preference.siteId, queryState = query.state)
  }

  fun refetch() {
    query.refetch()
  }

  suspend fun insertTemplate(
    template: EditorTemplateSheetTemplate,
    insert: suspend (ByteArray) -> Boolean,
  ): Result<Unit, Nothing> =
    loading({ loading -> insertingTemplateId = template.id.takeIf { loading } }) {
      val graph = loadTemplateGraph(template)
      if (!insert(graph)) {
        error("failed to insert template fragment")
      }
    }

  private suspend fun loadTemplateGraph(template: EditorTemplateSheetTemplate): ByteArray {
    val response =
      Apollo.query(EditorTemplateSheet_TemplateGraph_Query(slug = template.slug))
        .fetchPolicy(FetchPolicy.NetworkOnly)
        .execute()
    val graphError = response.errors?.firstOrNull()
    return when {
      response.exception != null -> throw response.exception!!
      graphError != null -> throw Exception(graphError.message)
      else -> response.data?.document?.state?.graph ?: error("missing template graph")
    }
  }
}

internal data class EditorTemplateSheetTemplate(val id: String, val title: String, val slug: String)

internal sealed interface EditorTemplateSheetContentState {
  data object Loading : EditorTemplateSheetContentState

  data object Error : EditorTemplateSheetContentState

  data object Empty : EditorTemplateSheetContentState

  data class Ready(val templates: List<EditorTemplateSheetTemplate>) :
    EditorTemplateSheetContentState
}

internal fun resolveEditorTemplateSheetContentState(
  siteId: String?,
  queryState: QueryState<EditorTemplateSheet_Query.Data>,
): EditorTemplateSheetContentState {
  if (siteId == null) {
    return EditorTemplateSheetContentState.Loading
  }

  return when (queryState) {
    is QueryState.Loading -> EditorTemplateSheetContentState.Loading
    is QueryState.Error -> EditorTemplateSheetContentState.Error
    is QueryState.Success -> {
      val templates =
        queryState.data.site.documentTemplates.map {
          EditorTemplateSheetTemplate(id = it.id, title = it.title, slug = it.entity.slug)
        }
      if (templates.isEmpty()) {
        EditorTemplateSheetContentState.Empty
      } else {
        EditorTemplateSheetContentState.Ready(templates)
      }
    }
  }
}
