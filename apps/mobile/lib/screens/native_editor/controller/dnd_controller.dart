import 'dart:async';
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

  bool _isDragging = false;
  bool get isDragging => _isDragging;

  Completer<void>? _imageCompleter;
  final ValueNotifier<
    ({ui.Image image, double scale, double offsetX, double offsetY, int pageIdx, ui.Offset initialPoint})?
  >
  dragUiImage = ValueNotifier(null);

  Future<void> prepareDragImage(int pageIdx, ui.Offset initialPoint) async {
    dragUiImage.value = null;
    _imageCompleter = Completer<void>();

    final result = await _tryRenderDragImage(pageIdx);
    if (result != null) {
      ui.decodeImageFromPixels(result.pixels, result.width, result.height, ui.PixelFormat.rgba8888, (image) {
        dragUiImage.value = (
          image: image,
          scale: result.scaleFactor,
          offsetX: result.offsetX,
          offsetY: result.offsetY,
          pageIdx: pageIdx,
          initialPoint: initialPoint,
        );
        _imageCompleter?.complete();
      });
      return;
    }

    final elements = controller.state.externalElements;
    double? minX, minY, maxX, maxY;

    for (final element in elements) {
      if (element.pageIdx == pageIdx && element.isSelected) {
        minX = minX == null ? element.bounds.x : math.min(minX, element.bounds.x);
        minY = minY == null ? element.bounds.y : math.min(minY, element.bounds.y);
        maxX = maxX == null
            ? (element.bounds.x + element.bounds.width)
            : math.max(maxX, element.bounds.x + element.bounds.width);
        maxY = maxY == null
            ? (element.bounds.y + element.bounds.height)
            : math.max(maxY, element.bounds.y + element.bounds.height);
      }
    }

    if (minX != null && minY != null && maxX != null && maxY != null) {
      final completer = Completer<ui.Image>();
      ui.decodeImageFromPixels(Uint8List.fromList([0, 0, 0, 0]), 1, 1, ui.PixelFormat.rgba8888, completer.complete);
      final transparentImage = await completer.future;

      dragUiImage.value = (
        image: transparentImage,
        scale: 1.0,
        offsetX: minX,
        offsetY: minY,
        pageIdx: pageIdx,
        initialPoint: initialPoint,
      );
    }

    _imageCompleter?.complete();
  }

  Future<NativeDragImageResult?> _tryRenderDragImage(int pageIdx) async {
    try {
      return editor.renderDragImage([pageIdx], pageIdx);
    } on EditorException catch (_) {
      return null;
    }
  }

  Future<DragItem?> createDragItem() async {
    if (_imageCompleter != null && !_imageCompleter!.isCompleted) {
      await _imageCompleter!.future;
    }

    final clipboardData = editor.getClipboardData();
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

    if (text != null && text.isNotEmpty) {
      item.add(Formats.plainText(text));
    }
    if (html != null && html.isNotEmpty) {
      item.add(Formats.htmlText(html));
    }

    return item;
  }

  void handleDragStart(int pageIdx, double x, double y, ui.Offset initialPoint) {
    _isDragging = true;
    unawaited(prepareDragImage(pageIdx, initialPoint));
    editor.dispatch({'type': 'dragStart', 'pageIdx': pageIdx, 'x': x, 'y': y});
  }

  void handleDragEnter() {
    editor.dispatch({'type': 'dragEnter'});
  }

  void handleDragLeave() {
    editor.dispatch({'type': 'dragLeave'});
  }

  void handleDragOver(int pageIdx, double x, double y) {
    editor.dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': x, 'y': y});
  }

  Future<void> handleDrop({
    required int pageIdx,
    required double x,
    required double y,
    required DropSession session,
  }) async {
    _isDragging = false;

    final item = session.items.firstOrNull;
    if (item == null) {
      _handleDragEnd();
      return;
    }

    // 내부 드래그인 경우
    final localData = item.localData;
    if (localData is Map && localData['isInternal'] == true) {
      editor.dispatch({
        'type': 'drop',
        'pageIdx': pageIdx,
        'x': x,
        'y': y,
        'text': localData['text'],
        'html': localData['html'],
        'fragment': localData['fragment'],
        'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
      });
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
        editor.dispatch({
          'type': 'drop',
          'pageIdx': pageIdx,
          'x': x,
          'y': y,
          'text': text,
          'html': null,
          'fragment': null,
          'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
        });
        return;
      }
    }

    _handleDragEnd();
  }

  void handleDragEnd() {
    _isDragging = false;
    _handleDragEnd();
  }

  void _handleDragEnd() {
    editor.dispatch({'type': 'dragEnd'});
  }
}
