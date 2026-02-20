import 'dart:ui' as ui;

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_thumbhash/flutter_thumbhash.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/gesture.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

class EditorDraggable extends StatelessWidget {
  const EditorDraggable({super.key, required this.child, required this.gesture});

  final Widget child;
  final GestureController gesture;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    return DragItemWidget(
      dragItemProvider: (request) {
        final renderBox = context.findRenderObject() as RenderBox?;
        if (renderBox == null) {
          return null;
        }
        final localPosition = renderBox.globalToLocal(request.location);
        final localDocumentY = localPosition.dy;
        final pointerX = gesture.getPointerX(localPosition.dx);

        final geo = scope.geometry;
        final offsets = geo.computeCumulativePageOffsets();
        var pageIdx = -1;
        var localY = 0.0;

        if (localDocumentY >= geo.titleAreaHeight) {
          final adjustedY = localDocumentY - geo.titleAreaHeight;
          var low = 0;
          var high = offsets.length - 1;
          while (low < high) {
            final mid = (low + high) ~/ 2;
            if (offsets[mid] <= adjustedY) {
              low = mid + 1;
            } else {
              high = mid;
            }
          }
          pageIdx = (low - 1).clamp(0, geo.pages.length - 1);
          localY = adjustedY - offsets[pageIdx];
        } else {
          // Inside Title Area
          pageIdx = -1;
          localY = localDocumentY;
        }

        final canDrag = scope.editor.isSelectionHit(pageIdx, pointerX, localY);

        if (!canDrag) {
          return null;
        }

        scope.dndController.handleDragStart(
          pageIdx,
          request.location.dx,
          localY,
          Offset(localPosition.dx, localDocumentY),
        );
        return scope.dndController.createDragItem();
      },
      allowedOperations: () => [DropOperation.copy, DropOperation.move],
      liftBuilder: (context, child) {
        return ValueListenableBuilder(
          valueListenable: scope.dndController.dragUiImage,
          builder: (context, imageRecord, _) {
            return _buildDragPreview(context, scope, imageRecord, child);
          },
        );
      },
      dragBuilder: (context, child) {
        return ValueListenableBuilder(
          valueListenable: scope.dndController.dragUiImage,
          builder: (context, imageRecord, _) {
            return _buildDragPreview(context, scope, imageRecord, child);
          },
        );
      },
      child: DraggableWidget(child: child),
    );
  }

  Widget _buildDragPreview(
    BuildContext context,
    ContentScope scope,
    ({ui.Image image, double scale, double offsetX, double offsetY, int pageIdx, ui.Offset initialPoint})? imageRecord,
    Widget child,
  ) {
    if (imageRecord == null) {
      return child;
    }

    final geo = scope.geometry;
    final offsets = geo.computeCumulativePageOffsets();
    final renderBox = context.findRenderObject() as RenderBox?;
    final viewportWidth = (renderBox?.hasSize ?? false) ? renderBox!.size.width : MediaQuery.sizeOf(context).width;
    final horizontalScrollOffset = scope.horizontalScrollController.hasSingleClient
        ? scope.horizontalScrollController.offset
        : 0.0;
    final contentStartX = geo.contentStartX(
      viewportWidth: viewportWidth,
      horizontalScrollOffset: horizontalScrollOffset,
    );

    final unionRect = Rect.fromLTWH(
      imageRecord.offsetX,
      imageRecord.offsetY,
      imageRecord.image.width / imageRecord.scale,
      imageRecord.image.height / imageRecord.scale,
    );

    return SnapshotSettings(
      translation: (rect, point) {
        final pageY = offsets[imageRecord.pageIdx];
        final targetX = contentStartX + unionRect.left;
        final targetY = geo.titleAreaHeight + pageY + unionRect.top;

        return Offset(targetX, targetY);
      },
      constraintsTransform: (constraints) => constraints.copyWith(
        minWidth: unionRect.width,
        maxWidth: unionRect.width,
        minHeight: unionRect.height,
        maxHeight: unionRect.height,
      ),
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          Positioned(
            left: 0,
            top: 0,
            width: unionRect.width,
            height: unionRect.height,
            child: RawImage(image: imageRecord.image, scale: imageRecord.scale),
          ),
          ..._buildExternalElements(context, scope, imageRecord, unionRect),
        ],
      ),
    );
  }

  List<Widget> _buildExternalElements(
    BuildContext context,
    ContentScope scope,
    ({ui.Image image, double scale, double offsetX, double offsetY, int pageIdx, ui.Offset initialPoint}) imageRecord,
    Rect unionRect,
  ) {
    final elements = scope.controller.state.externalElements;
    final uploadManager = NativeEditorToolbarScope.of(context).uploadManager;

    final widgets = <Widget>[];

    for (final element in elements) {
      if (!element.isSelected) {
        continue;
      }
      if (element.pageIdx != imageRecord.pageIdx) {
        continue;
      }

      final widget = element.data.mapOrNull(
        image: (imageData) {
          final localUploadId = uploadManager.localImageUploadIds[element.nodeId];
          final currentUploadId = imageData.uploadId ?? localUploadId;

          final asset = imageData.id != null ? uploadManager.imageAssets[imageData.id] : null;
          final inflight = currentUploadId != null ? uploadManager.inflightImages[currentUploadId] : null;

          if (asset == null && inflight == null) {
            return null;
          }

          final displayWidth = element.bounds.width * imageData.proportion;
          final xOffset = (element.bounds.width - displayWidth) / 2;

          final globalX = element.bounds.x + xOffset;
          final globalY = element.bounds.y;

          final destX = globalX - unionRect.left;
          final destY = globalY - unionRect.top;
          final destW = displayWidth;
          final destH = element.bounds.height;

          return Positioned(
            left: destX,
            top: destY,
            width: destW,
            height: destH,
            child: _buildImage(context, asset, inflight),
          );
        },
      );

      if (widget != null) {
        widgets.add(widget);
      }
    }

    return widgets;
  }

  Widget _buildImage(BuildContext context, ImageAsset? asset, InflightImage? inflight) {
    if (asset != null) {
      return CachedNetworkImage(
        imageUrl: asset.url,
        fit: BoxFit.cover,
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
        errorWidget: (context, url, error) => const SizedBox.shrink(),
      );
    }
    if (inflight != null) {
      return Image.memory(inflight.bytes, fit: BoxFit.cover);
    }
    return const SizedBox.shrink();
  }
}
