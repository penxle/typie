package co.typie.domain.editor.compose

import android.graphics.PixelFormat
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView

@Composable
internal actual fun Surface(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
) {
  val callback =
    remember(onAttach, onDetach, onResize) {
      var currentHandle = 0L

      object : SurfaceHolder.Callback {
        override fun surfaceCreated(holder: SurfaceHolder) {
          currentHandle = NativeWindowBridge.fromSurface(holder.surface)
          onAttach(currentHandle)
        }

        override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
          onResize()
        }

        override fun surfaceDestroyed(holder: SurfaceHolder) {
          onDetach()
          if (currentHandle != 0L) {
            NativeWindowBridge.release(currentHandle)
            currentHandle = 0L
          }
        }
      }
    }

  AndroidView(
    factory = { context ->
      SurfaceView(context).also {
        it.holder.setFormat(PixelFormat.TRANSLUCENT)
        it.setZOrderOnTop(true)
        it.holder.addCallback(callback)
      }
    },
    modifier = modifier,
    onRelease = { view -> view.holder.removeCallback(callback) },
  )
}
