package co.typie.screen.editor_settings

import androidx.lifecycle.ViewModel
import co.typie.service.EditorPreferencesService
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class EditorSettingsViewModel(
  val editorPreferencesService: EditorPreferencesService,
) : ViewModel()
