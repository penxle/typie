package co.typie

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import co.typie.dev.SystemChrome
import co.typie.shell.RootShell
import co.typie.ui.theme.AppTheme
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import com.hashsequence.coilresvg.ResvgDecoder

@Composable
@Preview
fun App() {
  setSingletonImageLoaderFactory { context ->
    ImageLoader.Builder(context)
      .components {
        add(ResvgDecoder.Factory())
      }
      .build()
  }

  AppTheme {
    SystemChrome {
      RootShell()
    }
  }
}
