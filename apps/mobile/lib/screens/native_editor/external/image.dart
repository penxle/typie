import 'dart:async';
import 'dart:io';
import 'dart:math' as math;

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_thumbhash/flutter_thumbhash.dart';
import 'package:path_provider/path_provider.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/persist_blob_as_image.req.gql.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';
import 'package:typie/services/blob.dart';

const _imageMinWidth = 100.0;
const _imageMinProportion = 0.1;
const _imageMaxProportion = 1.0;
const _imageProportionEpsilon = 0.001;
const _resizeHandleTouchWidth = 24.0;
const _resizeHandleVisualWidth = 8.0;
const _resizeHandleHorizontalInset = 10.0;
const _resizeHandleMaxHeight = 72.0;

class ImageWidget extends HookWidget {
  const ImageWidget({required this.element, super.key});

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
    final originalWidth = asset?.width.toDouble() ?? inflight?.width.toDouble() ?? 0;

    final resizeDraft = useState<_ImageResizeDraft?>(null);
    final activeResize = resizeDraft.value;
    final isResizing = activeResize != null;
    final displayProportion = activeResize?.proportion ?? proportion;
    final interactionController = EditorInteractionControllerScope.of(context);

    final imageWidth = _resolveImageWidth(boundsWidth, displayProportion, originalWidth);

    final processedUploadId = useRef<String?>(null);

    void startAuxiliaryGesture(AuxiliaryGestureKind kind) {
      interactionController.startAuxiliaryGesture(kind);
    }

    void updateAuxiliaryGesture(AuxiliaryGestureKind kind) {
      interactionController.updateAuxiliaryGesture(kind);
    }

    void endAuxiliaryGesture() {
      interactionController.endAuxiliaryGesture();
    }

    useEffect(() {
      if (currentUploadId != null && currentUploadId != processedUploadId.value && inflight != null && asset == null) {
        processedUploadId.value = currentUploadId;
        unawaited(_processUpload(scope, uploadManager, blob, client, context, currentUploadId, inflight));
      }
      return null;
    }, [currentUploadId, inflight, asset]);

    useEffect(() {
      if (!isResizing) {
        return null;
      }

      final shouldReset = !element.isSelected || !hasImage || boundsWidth <= 0;
      if (shouldReset) {
        endAuxiliaryGesture();
        resizeDraft.value = null;
      }
      return null;
    }, [isResizing, element.isSelected, hasImage, boundsWidth]);

    useEffect(() => endAuxiliaryGesture, const []);

    if (!hasImage) {
      return _buildPlaceholder(context);
    }

    final imageHeight = _calculateHeight(asset, inflight, imageWidth);
    final imageLeft = _maxDouble(0, (boundsWidth - imageWidth) / 2);
    final handleHeight = math.min(_resizeHandleMaxHeight, imageHeight / 3);
    final handleTop = _maxDouble(0, (imageHeight - handleHeight) / 2);
    final maxHandleLeft = _maxDouble(0, boundsWidth - _resizeHandleTouchWidth);
    final leftHandleLeft = _clampDouble(
      imageLeft + _resizeHandleHorizontalInset - _resizeHandleTouchWidth / 2,
      0,
      maxHandleLeft,
    );
    final rightHandleLeft = _clampDouble(
      imageLeft + imageWidth - _resizeHandleHorizontalInset - _resizeHandleTouchWidth / 2,
      0,
      maxHandleLeft,
    );

    void beginResize(PointerDownEvent event, {required bool reverse}) {
      if (resizeDraft.value != null || boundsWidth <= 0) {
        return;
      }

      startAuxiliaryGesture(AuxiliaryGestureKind.imageResize);

      final startWidth = _resolveImageWidth(boundsWidth, displayProportion, originalWidth);
      resizeDraft.value = _ImageResizeDraft(
        pointer: event.pointer,
        reverse: reverse,
        startX: event.position.dx,
        startWidth: startWidth,
        proportion: _clampProportion(displayProportion),
      );
      unawaited(HapticFeedback.lightImpact());
    }

    void updateResize(PointerMoveEvent event) {
      final current = resizeDraft.value;
      if (current == null || event.pointer != current.pointer || boundsWidth <= 0) {
        return;
      }

      final dx = (event.position.dx - current.startX) * (current.reverse ? -1 : 1);
      final nextWidth = _clampWidth(current.startWidth + dx * 2, boundsWidth, originalWidth);
      final nextProportion = _clampProportion(nextWidth / boundsWidth);
      updateAuxiliaryGesture(AuxiliaryGestureKind.imageResize);

      resizeDraft.value = current.copyWith(proportion: nextProportion);
    }

    void endResize(PointerEvent event) {
      final current = resizeDraft.value;
      if (current == null || event.pointer != current.pointer) {
        return;
      }

      endAuxiliaryGesture();
      resizeDraft.value = null;
      unawaited(HapticFeedback.lightImpact());

      final nextProportion = _clampProportion(current.proportion);
      if ((nextProportion - proportion).abs() <= _imageProportionEpsilon) {
        return;
      }

      scope.dispatch({'type': 'setImageProportion', 'nodeId': element.nodeId, 'proportion': nextProportion});
      scope.controller.scrollIntoView();
    }

    return SizedBox(
      width: boundsWidth,
      child: Stack(
        clipBehavior: Clip.none,
        alignment: Alignment.topCenter,
        children: [
          SizedBox(width: imageWidth, height: imageHeight, child: _buildImage(asset, inflight)),
          if (isUploading)
            Positioned(
              left: imageLeft,
              top: 0,
              width: imageWidth,
              height: imageHeight,
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
          if (element.isSelected && imageWidth > 0 && handleHeight > 0)
            Positioned(
              left: leftHandleLeft,
              top: handleTop,
              width: _resizeHandleTouchWidth,
              height: handleHeight,
              child: _ImageResizeHandle(
                active: isResizing,
                onPointerDown: (event) => beginResize(event, reverse: true),
                onPointerMove: updateResize,
                onPointerUp: endResize,
                onPointerCancel: endResize,
              ),
            ),
          if (element.isSelected && imageWidth > 0 && handleHeight > 0)
            Positioned(
              left: rightHandleLeft,
              top: handleTop,
              width: _resizeHandleTouchWidth,
              height: handleHeight,
              child: _ImageResizeHandle(
                active: isResizing,
                onPointerDown: (event) => beginResize(event, reverse: false),
                onPointerMove: updateResize,
                onPointerUp: endResize,
                onPointerCancel: endResize,
              ),
            ),
        ],
      ),
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

  double _resolveImageWidth(double boundsWidth, double proportion, double originalWidth) {
    if (boundsWidth <= 0) {
      return 0;
    }
    final safeProportion = _clampProportion(proportion);
    return _clampWidth(boundsWidth * safeProportion, boundsWidth, originalWidth);
  }

  double _clampWidth(double width, double boundsWidth, double originalWidth) {
    if (boundsWidth <= 0) {
      return 0;
    }

    final maxWidth = originalWidth > 0 ? math.min(originalWidth, boundsWidth) : boundsWidth;
    final requestedMin = math.max(boundsWidth * _imageMinProportion, _imageMinWidth);
    final minWidth = math.min(requestedMin, maxWidth);
    return _clampDouble(width, minWidth, maxWidth);
  }

  double _clampProportion(double value) {
    return _clampDouble(value, _imageMinProportion, _imageMaxProportion);
  }

  double _maxDouble(double a, double b) {
    return a >= b ? a : b;
  }

  double _clampDouble(double value, double min, double max) {
    if (value < min) {
      return min;
    }
    if (value > max) {
      return max;
    }
    return value;
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
            Expanded(
              child: Text(
                '이미지',
                style: TextStyle(fontSize: 14, color: context.colors.textDisabled),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ImageResizeHandle extends StatelessWidget {
  const _ImageResizeHandle({
    required this.active,
    required this.onPointerDown,
    required this.onPointerMove,
    required this.onPointerUp,
    required this.onPointerCancel,
  });

  final bool active;
  final ValueChanged<PointerDownEvent> onPointerDown;
  final ValueChanged<PointerMoveEvent> onPointerMove;
  final ValueChanged<PointerEvent> onPointerUp;
  final ValueChanged<PointerEvent> onPointerCancel;

  @override
  Widget build(BuildContext context) {
    return RawGestureDetector(
      behavior: HitTestBehavior.opaque,
      gestures: {
        EagerGestureRecognizer: GestureRecognizerFactoryWithHandlers<EagerGestureRecognizer>(
          EagerGestureRecognizer.new,
          (EagerGestureRecognizer instance) {},
        ),
      },
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: onPointerDown,
        onPointerMove: onPointerMove,
        onPointerUp: onPointerUp,
        onPointerCancel: onPointerCancel,
        child: Align(
          child: CustomPaint(
            painter: _ImageResizeHandlePainter(active: active),
            child: const SizedBox(height: double.infinity, width: _resizeHandleVisualWidth),
          ),
        ),
      ),
    );
  }
}

class _ImageResizeHandlePainter extends CustomPainter {
  const _ImageResizeHandlePainter({required this.active});

  final bool active;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 0 || size.height <= 0) {
      return;
    }

    final paint = Paint()
      ..color = active ? const Color.fromRGBO(255, 255, 255, 0.75) : const Color.fromRGBO(255, 255, 255, 0.55)
      ..blendMode = BlendMode.difference
      ..isAntiAlias = true;

    final radius = Radius.circular(size.width / 2);
    canvas.drawRRect(RRect.fromRectAndRadius(Offset.zero & size, radius), paint);
  }

  @override
  bool shouldRepaint(covariant _ImageResizeHandlePainter oldDelegate) {
    return oldDelegate.active != active;
  }
}

class _ImageResizeDraft {
  const _ImageResizeDraft({
    required this.pointer,
    required this.reverse,
    required this.startX,
    required this.startWidth,
    required this.proportion,
  });

  final int pointer;
  final bool reverse;
  final double startX;
  final double startWidth;
  final double proportion;

  _ImageResizeDraft copyWith({double? proportion}) {
    return _ImageResizeDraft(
      pointer: pointer,
      reverse: reverse,
      startX: startX,
      startWidth: startWidth,
      proportion: proportion ?? this.proportion,
    );
  }
}
