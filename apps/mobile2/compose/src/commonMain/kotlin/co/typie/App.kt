@file:OptIn(ExperimentalCoilApi::class)

package co.typie

import androidx.compose.runtime.Composable
import co.typie.dev.SystemChrome
import co.typie.ext.ScrollGestureLockScope
import co.typie.shell.RootShell
import co.typie.ui.component.popover.PopoverOutsideTapHost
import co.typie.ui.theme.AppTheme
import coil3.ImageLoader
import coil3.annotation.ExperimentalCoilApi
import coil3.compose.setSingletonImageLoaderFactory
import coil3.network.ktor3.KtorNetworkFetcherFactory
import coil3.request.crossfade
import com.hashsequence.coilresvg.ResvgDecoder
import io.ktor.client.HttpClient
import org.koin.compose.koinInject

@Composable
fun App() {
  val httpClient = koinInject<HttpClient>()

  setSingletonImageLoaderFactory { context ->
    ImageLoader.Builder(context)
      .crossfade(true)
      .components {
        add(KtorNetworkFetcherFactory(httpClient))
        add(ResvgDecoder.Factory())
      }
      .build()
  }

  AppTheme {
    SystemChrome {
      ScrollGestureLockScope {
        PopoverOutsideTapHost {
          RootShell()
        }
      }
    }
  }
}
