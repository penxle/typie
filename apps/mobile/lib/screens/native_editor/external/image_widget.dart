import 'dart:async';
import 'dart:io';

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_thumbhash/flutter_thumbhash.dart';
import 'package:path_provider/path_provider.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/persist_blob_as_image.req.gql.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/upload_manager.dart';
import 'package:typie/services/blob.dart';

class ExternalImageWidget extends HookWidget {
  const ExternalImageWidget({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;
    final blob = useService<Blob>();
    final client = useService<GraphQLClient>();

    useListenable(uploadManager);

    final imageData = element.data as ImageElementData;

    final localUploadId = uploadManager.localImageUploadIds[element.nodeId];
    final currentUploadId = imageData.uploadId ?? localUploadId;

    final asset = imageData.id != null ? uploadManager.imageAssets[imageData.id] : null;
    final inflight = currentUploadId != null ? uploadManager.inflightImages[currentUploadId] : null;

    final hasImage = asset != null || inflight != null;
    final isUploading = inflight != null && asset == null;

    final proportion = imageData.proportion;
    final boundsWidth = element.bounds.width;
    final imageWidth = boundsWidth * proportion;

    final processedUploadId = useRef<String?>(null);

    useEffect(() {
      if (currentUploadId != null && currentUploadId != processedUploadId.value && inflight != null && asset == null) {
        processedUploadId.value = currentUploadId;
        unawaited(_processUpload(scope, uploadManager, blob, client, context, currentUploadId, inflight));
      }
      return null;
    }, [currentUploadId, inflight, asset]);

    if (!hasImage) {
      return _buildPlaceholder(context);
    }

    final imageHeight = _calculateHeight(asset, inflight, imageWidth);

    return Stack(
      children: [
        SizedBox(width: imageWidth, height: imageHeight, child: _buildImage(asset, inflight)),
        if (isUploading)
          Positioned.fill(
            child: Container(
              color: Colors.white.withValues(alpha: 0.5),
              child: Center(
                child: SizedBox(
                  width: 24,
                  height: 24,
                  child: CircularProgressIndicator(strokeWidth: 2, color: context.colors.textDisabled),
                ),
              ),
            ),
          ),
      ],
    );
  }

  Future<void> _processUpload(
    NativeEditorToolbarScope scope,
    UploadManager uploadManager,
    Blob blob,
    GraphQLClient client,
    BuildContext context,
    String uploadId,
    InflightImage inflight,
  ) async {
    try {
      final tempDir = await getTemporaryDirectory();
      final tempFile = File('${tempDir.path}/upload_$uploadId.jpg');
      await tempFile.writeAsBytes(inflight.bytes);

      final path = await blob.upload(tempFile);
      final result = await client.request(
        GNativeEditorScreen_PersistBlobAsImage_MutationReq((b) => b..vars.input.path = path),
      );

      final image = result.persistBlobAsImage;
      final asset = ImageAsset(
        id: image.id,
        url: image.url,
        width: image.width,
        height: image.height,
        ratio: image.ratio,
        placeholder: image.placeholder,
      );
      uploadManager.completeImageUpload(uploadId: uploadId, nodeId: element.nodeId, asset: asset);

      scope.dispatch({'type': 'setImageId', 'nodeId': element.nodeId, 'imageId': image.id});

      await tempFile.delete();
    } catch (err) {
      uploadManager.failImageUpload(uploadId: uploadId, nodeId: element.nodeId);
      if (context.mounted) {
        context.toast(ToastType.error, '이미지 업로드에 실패했습니다');
      }
    }
  }

  double _calculateHeight(ImageAsset? asset, InflightImage? inflight, double width) {
    if (asset != null) {
      return width / asset.ratio;
    }
    if (inflight != null && inflight.width > 0) {
      return width * (inflight.height / inflight.width);
    }
    return 0;
  }

  Widget _buildImage(ImageAsset? asset, InflightImage? inflight) {
    if (asset != null) {
      return CachedNetworkImage(
        imageUrl: asset.url,
        fit: BoxFit.cover,
        fadeInDuration: const Duration(milliseconds: 150),
        placeholder: (context, url) {
          if (asset.placeholder != null) {
            try {
              final thumbHash = ThumbHash.fromBase64(asset.placeholder!);
              return Image(image: thumbHash.toImage(), fit: BoxFit.cover);
            } catch (_) {
              return const SizedBox.shrink();
            }
          }
          return const SizedBox.shrink();
        },
      );
    }
    if (inflight != null) {
      return Image.memory(inflight.bytes, fit: BoxFit.cover);
    }
    return const SizedBox.shrink();
  }

  Widget _buildPlaceholder(BuildContext context) {
    return Container(
      height: 48,
      decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(4)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            Icon(LucideLightIcons.image, size: 20, color: context.colors.textDisabled),
            const SizedBox(width: 12),
            Text('이미지', style: TextStyle(fontSize: 14, color: context.colors.textDisabled)),
          ],
        ),
      ),
    );
  }
}
