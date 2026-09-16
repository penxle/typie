package co.typie.editor.interaction.gestures

import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.ContextThemeWrapper
import android.widget.EditText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode

@Composable
internal actual fun rememberEditorSelectionHandleImages():
  Map<EditorSelectionHandleType, ImageBitmap> {
  val context = LocalContext.current
  val configuration = LocalConfiguration.current
  val themeMode = AppTheme.themeMode
  return remember(context, configuration, themeMode) {
    val handleConfiguration =
      Configuration(configuration).apply {
        val nightMode =
          if (themeMode == ResolvedThemeMode.Dark) Configuration.UI_MODE_NIGHT_YES
          else Configuration.UI_MODE_NIGHT_NO
        uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or nightMode
      }
    val themedContext =
      ContextThemeWrapper(context, 0).apply {
        applyOverrideConfiguration(handleConfiguration)
        theme.setTo(context.theme)
        theme.rebase()
      }
    val textView = EditText(themedContext)
    buildMap {
      for ((type, drawable) in
        listOf(
          EditorSelectionHandleType.Cursor to textView.textSelectHandle,
          EditorSelectionHandleType.From to textView.textSelectHandleLeft,
          EditorSelectionHandleType.To to textView.textSelectHandleRight,
        )) {
        if (drawable == null || drawable.intrinsicWidth <= 0 || drawable.intrinsicHeight <= 0)
          continue
        val image =
          Bitmap.createBitmap(
            drawable.intrinsicWidth,
            drawable.intrinsicHeight,
            Bitmap.Config.ARGB_8888,
          )
        drawable.mutate().apply {
          setBounds(0, 0, image.width, image.height)
          draw(Canvas(image))
        }
        put(type, image.asImageBitmap())
      }
    }
  }
}
