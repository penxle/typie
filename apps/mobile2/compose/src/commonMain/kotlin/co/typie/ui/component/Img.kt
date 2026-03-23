package co.typie.ui.component

import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import co.typie.graphql.fragment.Img_image
import co.typie.ui.skeleton.LocalSkeleton
import co.typie.ui.skeleton.SkeletonBone
import coil3.compose.AsyncImage
import kotlin.math.ceil
import kotlin.math.log2
import kotlin.math.pow

object Img {
  @Composable
  operator fun invoke(
    image: Img_image?,
    modifier: Modifier = Modifier,
    contentScale: ContentScale = ContentScale.Crop,
  ) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      SkeletonBone(modifier)
      return
    }

    if (image == null) return

    val density = LocalDensity.current

    BoxWithConstraints(modifier = modifier) {
      val maxDim = maxOf(constraints.maxWidth, constraints.maxHeight)
      val fetchSize = if (maxDim > 0 && maxDim != Constraints.Infinity) {
        2.0.pow(ceil(log2(maxDim.toDouble() * density.density))).toInt()
      } else {
        0
      }

      AsyncImage(
        model = if (fetchSize > 0) "${image.url}?s=$fetchSize&q=75" else image.url,
        contentDescription = null,
        modifier = Modifier.fillMaxSize(),
        contentScale = contentScale,
      )
    }
  }


  @Composable
  operator fun invoke(
    url: String,
    modifier: Modifier = Modifier,
    color: Color? = null,
    contentScale: ContentScale = ContentScale.Crop,
  ) {
    val skeleton = LocalSkeleton.current
    if (skeleton.enabled) {
      SkeletonBone(modifier)
      return
    }

    AsyncImage(
      model = url,
      contentDescription = null,
      modifier = modifier,
      colorFilter = color?.let { ColorFilter.tint(it) },
      contentScale = contentScale,
    )
  }
}
