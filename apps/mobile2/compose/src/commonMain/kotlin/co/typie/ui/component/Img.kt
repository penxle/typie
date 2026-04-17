package co.typie.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.painter.ColorPainter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import co.typie.graphql.fragment.Img_image
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.skeletonBone
import coil3.compose.AsyncImage
import coil3.compose.SubcomposeAsyncImage
import io.ktor.http.Url
import kotlin.math.ceil
import kotlin.math.log2
import kotlin.math.pow

object Img {
  @Composable
  operator fun invoke(
    image: Img_image?,
    modifier: Modifier = Modifier,
    contentScale: ContentScale = ContentScale.Crop,
    placeholderColor: Color? = null,
    placeholder: @Composable (() -> Unit)? = null,
  ) {
    val isSkeleton = LocalSkeleton.current.enabled
    Box(modifier.skeletonBone(RectangleShape)) {
      if (isSkeleton) return@Box
      if (image == null) return@Box

      val density = LocalDensity.current
      BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
        val maxDim = maxOf(constraints.maxWidth, constraints.maxHeight)
        val fetchSize =
          if (maxDim > 0 && maxDim != Constraints.Infinity) {
            2.0.pow(ceil(log2(maxDim.toDouble() * density.density))).toInt()
          } else {
            0
          }

        val model = if (fetchSize > 0) "${image.url}?s=$fetchSize&q=75" else image.url

        if (placeholder != null) {
          PlaceholderAsyncImage(
            model = model,
            modifier = Modifier.fillMaxSize(),
            contentScale = contentScale,
            placeholder = placeholder,
          )
        } else {
          AsyncImage(
            model = model,
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = contentScale,
            placeholder = placeholderColor?.let { ColorPainter(it) },
          )
        }
      }
    }
  }

  @Composable
  operator fun invoke(
    url: String,
    modifier: Modifier = Modifier,
    color: Color? = null,
    contentScale: ContentScale = ContentScale.Crop,
    placeholderColor: Color? = null,
    placeholder: @Composable (() -> Unit)? = null,
  ) {
    val isSkeleton = LocalSkeleton.current.enabled
    Box(modifier.skeletonBone(RectangleShape)) {
      if (isSkeleton) return@Box

      if (placeholder != null) {
        PlaceholderAsyncImage(
          model = url,
          modifier = Modifier.fillMaxSize(),
          contentScale = contentScale,
          colorFilter = color?.let { ColorFilter.tint(it) },
          placeholder = placeholder,
        )
      } else {
        AsyncImage(
          model = url,
          contentDescription = null,
          modifier = Modifier.fillMaxSize(),
          colorFilter = color?.let { ColorFilter.tint(it) },
          contentScale = contentScale,
          placeholder = placeholderColor?.let { ColorPainter(it) },
        )
      }
    }
  }

  @Composable
  operator fun invoke(
    url: Url,
    modifier: Modifier = Modifier,
    color: Color? = null,
    contentScale: ContentScale = ContentScale.Crop,
    placeholderColor: Color? = null,
    placeholder: @Composable (() -> Unit)? = null,
  ) {
    val isSkeleton = LocalSkeleton.current.enabled
    Box(modifier.skeletonBone(RectangleShape)) {
      if (isSkeleton) return@Box

      if (placeholder != null) {
        PlaceholderAsyncImage(
          model = url.toString(),
          modifier = Modifier.fillMaxSize(),
          contentScale = contentScale,
          colorFilter = color?.let { ColorFilter.tint(it) },
          placeholder = placeholder,
        )
      } else {
        AsyncImage(
          model = url.toString(),
          contentDescription = null,
          modifier = Modifier.fillMaxSize(),
          colorFilter = color?.let { ColorFilter.tint(it) },
          contentScale = contentScale,
          placeholder = placeholderColor?.let { ColorPainter(it) },
        )
      }
    }
  }
}

@Composable
private fun PlaceholderAsyncImage(
  model: Any,
  modifier: Modifier,
  contentScale: ContentScale,
  colorFilter: ColorFilter? = null,
  placeholder: @Composable () -> Unit,
) {
  SubcomposeAsyncImage(
    model = model,
    contentDescription = null,
    modifier = modifier,
    contentScale = contentScale,
    colorFilter = colorFilter,
    loading = { placeholder() },
  )
}
