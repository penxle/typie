import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class DndController {
  DndController({required this.editor, required this.controller});

  final NativeEditor editor;
  final EditorController controller;

  Completer<void>? _imageCompleter;
  final ValueNotifier<
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
  >
  dragUiImage = ValueNotifier(null);

  Future<void> prepareDragImage(int pageIdx, double startX, double startY, ui.Offset initialPoint) async {
    dragUiImage.value = null;
    _imageCompleter = Completer<void>();

    // TODO: 안드로이드 드래그 이미지 지원
    final result = Platform.isAndroid ? null : await _tryRenderDragImage(pageIdx);
    if (result != null) {
      final decoded = await _decodeImageSafe(result.pixels, result.width, result.height);
      if (decoded != null) {
        dragUiImage.value = (
          image: decoded,
          scale: result.scaleFactor,
          offsetX: result.offsetX,
          offsetY: result.offsetY,
          pageIdx: pageIdx,
          startX: startX,
          startY: startY,
          initialPoint: initialPoint,
        );
        if (!(_imageCompleter?.isCompleted ?? true)) {
          _imageCompleter?.complete();
        }
        return;
      }
    }

    final elements = controller.state.externalElements;
    double? minX, minY;

    for (final element in elements) {
      if (element.pageIdx == pageIdx && element.isSelected) {
        minX = minX == null ? element.bounds.x : math.min(minX, element.bounds.x);
        minY = minY == null ? element.bounds.y : math.min(minY, element.bounds.y);
      }
    }

    final transparentImage = await _decodeImageSafe(Uint8List.fromList([0, 0, 0, 0]), 1, 1);
    if (transparentImage == null) {
      if (!(_imageCompleter?.isCompleted ?? true)) {
        _imageCompleter?.complete();
      }
      return;
    }

    dragUiImage.value = (
      image: transparentImage,
      scale: 1.0,
      offsetX: minX ?? 0,
      offsetY: minY ?? 0,
      pageIdx: pageIdx,
      startX: startX,
      startY: startY,
      initialPoint: initialPoint,
    );

    if (!(_imageCompleter?.isCompleted ?? true)) {
      _imageCompleter?.complete();
    }
  }

  Future<ui.Image?> _decodeImageSafe(Uint8List pixels, int width, int height) async {
    if (width <= 0 || height <= 0) {
      return null;
    }

    final expected = width * height * 4;
    if (pixels.lengthInBytes != expected) {
      return null;
    }

    final completer = Completer<ui.Image>();
    try {
      ui.decodeImageFromPixels(pixels, width, height, ui.PixelFormat.rgba8888, completer.complete);
      return await completer.future;
    } catch (_) {
      return null;
    }
  }

  Future<NativeDragImageResult?> _tryRenderDragImage(int pageIdx) async {
    try {
      return editor.renderDragImage([pageIdx], pageIdx);
    } on EditorException catch (_) {
      return null;
    }
  }

  Future<DragItem?> createDragItem() async {
    final pendingImage = _imageCompleter;
    if (pendingImage != null && !pendingImage.isCompleted) {
      await pendingImage.future.timeout(const Duration(milliseconds: 250), onTimeout: () {});
    }

    final Map<String, dynamic>? clipboardData;
    try {
      clipboardData = editor.getClipboardData();
    } on EditorException catch (_) {
      return null;
    }

    if (clipboardData == null) {
      return null;
    }

    final text = clipboardData['text'] as String?;
    final html = clipboardData['html'] as String?;
    final fragment = clipboardData['fragment'] as String?;

    if ((text == null || text.isEmpty) && (html == null || html.isEmpty) && (fragment == null || fragment.isEmpty)) {
      return null;
    }

    final item = DragItem(localData: {'text': text, 'html': html, 'fragment': fragment, 'isInternal': true});
    var hasTransferFormat = false;

    if (text != null && text.isNotEmpty) {
      item.add(Formats.plainText(text));
      hasTransferFormat = true;
    }
    if (html != null && html.isNotEmpty) {
      if (!hasTransferFormat) {
        // Android requires a plain text fallback when providing HTML.
        item.add(Formats.plainText(''));
        hasTransferFormat = true;
      }
      item.add(Formats.htmlText(html));
      hasTransferFormat = true;
    }

    if (!hasTransferFormat) {
      item.add(Formats.plainText(''));
    }

    return item;
  }

  void handleDragStart(int pageIdx, double x, double y, ui.Offset initialPoint) {
    unawaited(prepareDragImage(pageIdx, x, y, initialPoint));
    controller
      ..dispatch({'type': 'dragStart', 'pageIdx': pageIdx, 'x': x, 'y': y})
      ..scrollIntoView();
  }

  void handleDragEnter() {
    controller.dispatch({'type': 'dragEnter'});
  }

  void handleDragLeave() {
    controller.dispatch({'type': 'dragLeave'});
  }

  void handleDragOver(int pageIdx, double x, double y) {
    if (pageIdx < 0) {
      controller.dispatch({'type': 'dragLeave'});
      return;
    }
    controller.dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': x, 'y': y});
  }

  Future<void> handleDrop({
    required int pageIdx,
    required double x,
    required double y,
    required DropSession session,
  }) async {
    final item = session.items.firstOrNull;
    if (item == null) {
      _handleDragEnd();
      return;
    }

    // 내부 드래그인 경우
    final localData = item.localData;
    if (localData is Map && localData['isInternal'] == true) {
      controller
        ..dispatch({
          'type': 'drop',
          'pageIdx': pageIdx,
          'x': x,
          'y': y,
          'text': localData['text'],
          'html': localData['html'],
          'fragment': localData['fragment'],
          'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
        })
        ..scrollIntoView();
      return;
    }

    // 외부 드래그인 경우
    final reader = item.dataReader;
    if (reader == null) {
      _handleDragEnd();
      return;
    }

    if (reader.canProvide(Formats.plainText)) {
      final completer = Completer<String?>();
      reader.getValue<String>(
        Formats.plainText,
        completer.complete,
        onError: (error) {
          completer.complete(null);
        },
      );

      final text = await completer.future;
      if (text != null && text.isNotEmpty) {
        controller
          ..dispatch({
            'type': 'drop',
            'pageIdx': pageIdx,
            'x': x,
            'y': y,
            'text': text,
            'html': null,
            'fragment': null,
            'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
          })
          ..scrollIntoView();
        return;
      }
    }

    _handleDragEnd();
  }

  void handleDragEnd() {
    dragUiImage.value = null;
    if (!(_imageCompleter?.isCompleted ?? true)) {
      _imageCompleter?.complete();
    }
    _handleDragEnd();
  }

  void _handleDragEnd() {
    controller.dispatch({'type': 'dragEnd'});
  }

  void dispose() {
    dragUiImage.dispose();
  }
}
