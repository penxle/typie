import 'dart:async';
import 'dart:ui' as ui;

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_thumbhash/flutter_thumbhash.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

class EditorDraggable extends StatelessWidget {
  const EditorDraggable({super.key, required this.child, required this.interactionController});

  final Widget child;
  final EditorInteractionController interactionController;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);

    bool isSelectionDraggable(Offset globalPosition) {
      return interactionController.resolveSelectionDrag(globalPosition) != null;
    }

    return DragItemWidget(
      dragItemProvider: (request) async {
        final resolved = interactionController.resolveSelectionDrag(request.location);
        if (resolved == null) {
          return null;
        }
        interactionController.startLocalDnd(resolved);

        var isSessionListenerAttached = false;
        late final VoidCallback onSessionCompleted;
        void clearSessionCompletedListener() {
          if (!isSessionListenerAttached) {
            return;
          }
          request.session.dragCompleted.removeListener(onSessionCompleted);
          isSessionListenerAttached = false;
        }

        onSessionCompleted = () {
          final op = request.session.dragCompleted.value;
          if (op == null) {
            return;
          }
          clearSessionCompletedListener();
          interactionController.onLocalDragCompleted(op);
        };
        request.session.dragCompleted.addListener(onSessionCompleted);
        isSessionListenerAttached = true;

        unawaited(HapticFeedback.lightImpact());
        final item = await scope.dndController.createDragItem();
        if (item == null) {
          clearSessionCompletedListener();
          interactionController.endDnd();
        }
        return item;
      },
      allowedOperations: () => [DropOperation.copy, DropOperation.move],
      liftBuilder: (context, _) {
        return ValueListenableBuilder(
          valueListenable: scope.dndController.dragUiImage,
          builder: (context, imageRecord, _) {
            return _buildDragPreview(context, scope, imageRecord);
          },
        );
      },
      dragBuilder: (context, _) {
        return ValueListenableBuilder(
          valueListenable: scope.dndController.dragUiImage,
          builder: (context, imageRecord, _) {
            return _buildDragPreview(context, scope, imageRecord);
          },
        );
      },
      child: DraggableWidget(isLocationDraggable: isSelectionDraggable, child: child),
    );
  }

  Widget _buildDragPreview(
    BuildContext context,
    ContentScope scope,
    ({
      ui.Image image,
      double scale,
      double offsetX,
      double offsetY,
      int pageIdx,
      double startX,
      double startY,
      ui.Offset initialPoint,
    })?
    imageRecord,
  ) {
    if (imageRecord == null) {
      return const SizedBox(width: 1, height: 1);
    }

    final geo = scope.geometry;
    final unionRect = Rect.fromLTWH(
      imageRecord.offsetX,
      imageRecord.offsetY,
      imageRecord.image.width / imageRecord.scale,
      imageRecord.image.height / imageRecord.scale,
    );
    final displayRect = Rect.fromLTWH(
      geo.toDisplayX(unionRect.left),
      geo.toDisplayY(unionRect.top),
      geo.toDisplayX(unionRect.width),
      geo.toDisplayY(unionRect.height),
    );
    final zoom = geo.effectiveZoom > 0 ? geo.effectiveZoom : 1.0;

    return SnapshotSettings(
      translation: (rect, point) {
        final pointerOffsetX = geo.toDisplayX(imageRecord.startX - unionRect.left);
        final pointerOffsetY = geo.toDisplayY(imageRecord.startY - unionRect.top);
        final targetX = imageRecord.initialPoint.dx - pointerOffsetX;
        final targetY = imageRecord.initialPoint.dy - pointerOffsetY;

        return Offset(targetX, targetY);
      },
      constraintsTransform: (constraints) => constraints.copyWith(
        minWidth: displayRect.width,
        maxWidth: displayRect.width,
        minHeight: displayRect.height,
        maxHeight: displayRect.height,
      ),
      child: SizedBox(
        width: displayRect.width,
        height: displayRect.height,
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            Positioned.fill(
              child: RawImage(image: imageRecord.image, scale: imageRecord.scale / zoom),
            ),
            ..._buildExternalElements(context, scope, imageRecord, unionRect, zoom),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildExternalElements(
    BuildContext context,
    ContentScope scope,
    ({
      ui.Image image,
      double scale,
      double offsetX,
      double offsetY,
      int pageIdx,
      double startX,
      double startY,
      ui.Offset initialPoint,
    })
    imageRecord,
    Rect unionRect,
    double zoom,
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

          final globalX = (element.bounds.x + xOffset) * zoom;
          final globalY = element.bounds.y * zoom;

          final destX = globalX - unionRect.left * zoom;
          final destY = globalY - unionRect.top * zoom;
          final destW = displayWidth * zoom;
          final destH = element.bounds.height * zoom;

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
