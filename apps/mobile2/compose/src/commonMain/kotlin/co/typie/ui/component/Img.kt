package co.typie.ui.component

import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import co.typie.ext.toPx
import co.typie.graphql.fragment.Img_image
import coil3.compose.AsyncImage
import kotlin.math.ceil
import kotlin.math.log2
import kotlin.math.pow

@Composable
fun Img(
  image: Img_image?,
  size: Dp,
  modifier: Modifier = Modifier,
  width: Dp = size,
  height: Dp = size,
  contentScale: ContentScale = ContentScale.Crop,
) {
  if (image == null) return

  val density = LocalDensity.current
  val fetchSize = 2.0.pow(ceil(log2((size.toPx(density) * density.density).toDouble()))).toInt()

  AsyncImage(
    model = "${image.url}?s=$fetchSize&q=75",
    contentDescription = null,
    modifier = modifier.size(width, height),
    contentScale = contentScale,
  )
}
