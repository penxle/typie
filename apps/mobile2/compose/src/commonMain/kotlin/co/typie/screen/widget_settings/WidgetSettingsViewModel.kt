package co.typie.screen.widget_settings

import androidx.lifecycle.ViewModel
import co.typie.service.EditorPreferencesService
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class WidgetSettingsViewModel(
  val editorPreferencesService: EditorPreferencesService,
) : ViewModel()
