package co.typie.editor

import androidx.compose.runtime.snapshotFlow
import co.typie.editor.ffi.RawTextReplacementRule
import co.typie.graphql.fragment.TextReplacementLoader_user
import co.typie.graphql.type.TextReplacementState
import co.typie.platform.PlatformModule
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

object TextReplacementLoader {
  fun watchTextReplacements(scope: CoroutineScope, user: () -> TextReplacementLoader_user?) {
    scope.launch {
      snapshotFlow(user).filterNotNull().distinctUntilChanged().collect { user ->
        withContext(Dispatchers.Default) {
          PlatformModule.editorHost.setTextReplacementRules(user.toTextReplacementRules())
        }
      }
    }
  }
}

internal fun TextReplacementLoader_user.toTextReplacementRules(): List<RawTextReplacementRule> =
  textReplacements.mapNotNull { item ->
    val preference = item.onTextReplacementPreference
    if (preference != null) {
      if (preference.state != TextReplacementState.ACTIVE) {
        return@mapNotNull null
      }
      RawTextReplacementRule(
        id = preference.textReplacement.id,
        matchPattern = preference.textReplacement.match,
        substitute = preference.textReplacement.substitute,
        regex = preference.textReplacement.regex,
      )
    } else {
      item.onTextReplacement?.let {
        RawTextReplacementRule(
          id = it.id,
          matchPattern = it.match,
          substitute = it.substitute,
          regex = it.regex,
        )
      }
    }
  }
