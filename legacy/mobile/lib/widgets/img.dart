import 'dart:math' as math;

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:skeletonizer/skeletonizer.dart';
import 'package:transparent_image/transparent_image.dart';
import 'package:typie/widgets/__generated__/img.data.gql.dart';

class Img extends HookWidget {
  const Img({
    super.key,
    required this.image,
    required this.size,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
    this.progressive = false,
  });

  final GImg_image? image;
  final double size;
  final double? width;
  final double? height;
  final BoxFit fit;
  final bool progressive;

  @override
  Widget build(BuildContext context) {
    final loading = useState(true);

    final fetchSize = math
        .pow(2, (math.log(size * MediaQuery.devicePixelRatioOf(context)) / math.log(2)).ceil())
        .toInt();

    final url = image != null ? '${image!.url}?s=$fetchSize&q=75' : '';

    return Skeleton.keep(
      child: Skeletonizer(
        enabled: loading.value,
        child: Image(
          image: image != null ? CachedNetworkImageProvider(url) : MemoryImage(kTransparentImage),
          width: width ?? size,
          height: height ?? size,
          fit: fit,
          frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
            if (image != null && wasSynchronouslyLoaded || frame != null) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                loading.value = false;
              });
            }

            return child;
          },
        ),
      ),
    );
  }
}
