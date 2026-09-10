package co.typie.screen.editor.editor.attachment

import androidx.compose.ui.platform.UriHandler
import co.typie.network.Http
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.platform.ShareAnchor
import co.typie.ui.component.toast.Toast
import kotlinx.coroutines.CancellationException

/** Uses the same browser/share-sheet download flow from the toolbar and context menu. */
internal suspend fun downloadEditorImage(
  url: String,
  anchor: ShareAnchor?,
  uriHandler: UriHandler,
  toast: Toast,
) {
  if (PlatformModule.platform == Platform.Desktop) {
    uriHandler.openUri(url)
    return
  }
  val downloaded =
    try {
      Http.downloadEditorAttachment(url = url, defaultFilenameStem = "image")
    } catch (error: CancellationException) {
      throw error
    } catch (_: Throwable) {
      toast.error("이미지를 내려받을 수 없어요.")
      return
    }
  val shared =
    try {
      PlatformModule.share.share(
        bytes = downloaded.bytes,
        filename = downloaded.filename,
        mimeType = downloaded.mimeType,
        anchor = anchor,
      )
    } catch (error: CancellationException) {
      throw error
    } catch (_: Throwable) {
      false
    }
  if (!shared) toast.error("이미지를 내보낼 수 없어요.")
}
