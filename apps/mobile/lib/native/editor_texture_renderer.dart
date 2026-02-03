import 'dart:async';

import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';

class _BatchRenderItem {
  _BatchRenderItem({
    required this.textureId,
    required this.editorPtr,
    required this.pageIndex,
    required this.width,
    required this.height,
    required this.completer,
  });

  final int textureId;
  final int editorPtr;
  final int pageIndex;
  final int width;
  final int height;
  final Completer<bool> completer;
}

class EditorTextureRenderer {
  EditorTextureRenderer({required this.editor});

  static const _channel = MethodChannel('co.typie.editor_texture');
  static final _pendingRenders = <_BatchRenderItem>[];
  static bool _flushScheduled = false;

  final NativeEditor editor;
  int? _textureId;
  int _currentWidth = 0;
  int _currentHeight = 0;

  int? get textureId => _textureId;
  int get width => _currentWidth;
  int get height => _currentHeight;

  Future<void> create(int pageIndex) async {
    final info = editor.getRenderInfo(pageIndex);
    if (info == null) {
      return;
    }

    _currentWidth = info.width;
    _currentHeight = info.height;

    _textureId = await _channel.invokeMethod<int>('create', {'width': _currentWidth, 'height': _currentHeight});
  }

  Future<bool> render(int pageIndex) async {
    if (_textureId == null) {
      return false;
    }

    final info = editor.getRenderInfo(pageIndex);
    if (info == null) {
      return false;
    }

    _currentWidth = info.width;
    _currentHeight = info.height;

    final completer = Completer<bool>();
    _pendingRenders.add(
      _BatchRenderItem(
        textureId: _textureId!,
        editorPtr: editor.handle.address,
        pageIndex: pageIndex,
        width: info.width,
        height: info.height,
        completer: completer,
      ),
    );

    if (!_flushScheduled) {
      _flushScheduled = true;
      scheduleMicrotask(_flushBatch);
    }

    return completer.future;
  }

  static Future<void> _flushBatch() async {
    _flushScheduled = false;

    final batch = List.of(_pendingRenders);
    _pendingRenders.clear();

    if (batch.isEmpty) {
      return;
    }

    try {
      await _channel.invokeMethod<void>('render', {
        'items': batch
            .map(
              (r) => <String, dynamic>{
                'textureId': r.textureId,
                'editorPtr': r.editorPtr,
                'pageIndex': r.pageIndex,
                'width': r.width,
                'height': r.height,
              },
            )
            .toList(),
      });
      for (final item in batch) {
        item.completer.complete(true);
      }
    } catch (_) {
      for (final item in batch) {
        item.completer.complete(false);
      }
    }
  }

  Future<void> dispose() async {
    final id = _textureId;
    if (id == null) {
      return;
    }

    _textureId = null;
    await _channel.invokeMethod<void>('dispose', {'textureId': id});
  }
}
