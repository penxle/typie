package co.typie.service

import co.typie.storage.Prefs

object EditorPreferencesService {
  const val TYPEWRITER_ENABLED_KEY = "typewriter_enabled"
  const val TYPEWRITER_POSITION_KEY = "typewriter_position"
  const val LINE_HIGHLIGHT_ENABLED_KEY = "line_highlight_enabled"
  const val AUTO_SURROUND_ENABLED_KEY = "auto_surround_enabled"
  const val CHARACTER_COUNT_FLOATING_ENABLED_KEY = "character_count_floating_enabled"
  const val WIDGET_AUTO_FADE_ENABLED_KEY = "widget_auto_fade_enabled"

  const val DEFAULT_TYPEWRITER_ENABLED = false
  const val DEFAULT_TYPEWRITER_POSITION = 0.5
  const val DEFAULT_LINE_HIGHLIGHT_ENABLED = true
  const val DEFAULT_AUTO_SURROUND_ENABLED = true
  const val DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED = false
  const val DEFAULT_WIDGET_AUTO_FADE_ENABLED = true

  var typewriterEnabled: Boolean by Prefs(TYPEWRITER_ENABLED_KEY, DEFAULT_TYPEWRITER_ENABLED)
  var typewriterPosition: Double by Prefs(TYPEWRITER_POSITION_KEY, DEFAULT_TYPEWRITER_POSITION)
  var lineHighlightEnabled: Boolean by
    Prefs(LINE_HIGHLIGHT_ENABLED_KEY, DEFAULT_LINE_HIGHLIGHT_ENABLED)
  var autoSurroundEnabled: Boolean by
    Prefs(AUTO_SURROUND_ENABLED_KEY, DEFAULT_AUTO_SURROUND_ENABLED)
  var characterCountFloatingEnabled: Boolean by
    Prefs(CHARACTER_COUNT_FLOATING_ENABLED_KEY, DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED)
  var widgetAutoFadeEnabled: Boolean by
    Prefs(WIDGET_AUTO_FADE_ENABLED_KEY, DEFAULT_WIDGET_AUTO_FADE_ENABLED)
}
