import 'dart:async';

import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/native/editor_render_coordinator.dart';

class _BatchRenderItem {
  _BatchRenderItem({
    required this.textureId,
    required this.editor,
    required this.editorPtr,
    required this.pageIndex,
    required this.width,
    required this.height,
    required this.requestToken,
    required this.completer,
  });

  final int textureId;
  final NativeEditor editor;
  final int editorPtr;
  final int pageIndex;
  final int width;
  final int height;
  final int requestToken;
  final Completer<bool> completer;
}

class EditorTextureRenderer {
  EditorTextureRenderer({required this.editor});

  static const _channel = MethodChannel('co.typie.editor_texture');
  static final _pendingRenders = <_BatchRenderItem>[];
  static final _latestRequestTokenByTexture = <int, int>{};
  static int _nextRequestToken = 1;
  static bool _flushScheduled = false;
  static bool _isFlushing = false;

  final NativeEditor editor;
  int? _textureId;
  int _currentWidth = 0;
  int _currentHeight = 0;

  int? get textureId => _textureId;
  int get width => _currentWidth;
  int get height => _currentHeight;

  bool _isRenderableSize(int width, int height) {
    return width > 0 && height > 0;
  }

  Future<void> create(int pageIndex) async {
    final info = editor.getRenderInfo(pageIndex);
    if (info == null) {
      return;
    }
    if (!_isRenderableSize(info.width, info.height)) {
      return;
    }

    _currentWidth = info.width;
    _currentHeight = info.height;

    _textureId = await _channel.invokeMethod<int>('create', {'width': _currentWidth, 'height': _currentHeight});
  }

  Future<bool> render(int pageIndex) async {
    if (_textureId == null || editor.isDisposed) {
      return false;
    }

    final editorPtr = editor.handle.address;
    if (EditorRenderCoordinator.isEditorDisposing(editorPtr)) {
      return false;
    }

    final info = editor.getRenderInfo(pageIndex);
    if (info == null) {
      return false;
    }
    if (!_isRenderableSize(info.width, info.height)) {
      return false;
    }

    _currentWidth = info.width;
    _currentHeight = info.height;

    final completer = Completer<bool>();
    final requestToken = _nextRequestToken++;
    _latestRequestTokenByTexture[_textureId!] = requestToken;
    _pendingRenders.add(
      _BatchRenderItem(
        textureId: _textureId!,
        editor: editor,
        editorPtr: editorPtr,
        pageIndex: pageIndex,
        width: info.width,
        height: info.height,
        requestToken: requestToken,
        completer: completer,
      ),
    );

    if (!_flushScheduled) {
      _flushScheduled = true;
      scheduleMicrotask(_flushBatchLoop);
    }

    return completer.future;
  }

  static Future<void> _flushBatchLoop() async {
    if (_isFlushing) {
      return;
    }
    _isFlushing = true;
    try {
      while (_pendingRenders.isNotEmpty) {
        final batch = List.of(_pendingRenders);
        _pendingRenders.clear();
        await _flushBatch(batch);
      }
    } finally {
      _isFlushing = false;
      _flushScheduled = false;
      if (_pendingRenders.isNotEmpty) {
        _flushScheduled = true;
        scheduleMicrotask(_flushBatchLoop);
      }
    }
  }

  static Future<void> _flushBatch(List<_BatchRenderItem> batch) async {
    if (batch.isEmpty) {
      return;
    }

    final latestByTexture = <int, _BatchRenderItem>{};
    for (final item in batch) {
      final previous = latestByTexture[item.textureId];
      if (previous == null) {
        latestByTexture[item.textureId] = item;
        continue;
      }
      if (item.requestToken >= previous.requestToken) {
        previous.completer.complete(false);
        latestByTexture[item.textureId] = item;
      } else {
        item.completer.complete(false);
      }
    }

    final submitted = <_BatchRenderItem>[];
    final payloadItems = <Map<String, dynamic>>[];

    for (final item in latestByTexture.values) {
      final latestToken = _latestRequestTokenByTexture[item.textureId];
      if (latestToken != item.requestToken) {
        item.completer.complete(false);
        continue;
      }

      if (item.editor.isDisposed || EditorRenderCoordinator.isEditorDisposing(item.editorPtr)) {
        item.completer.complete(false);
        continue;
      }

      NativeEditorRenderInfo? latestInfo;
      try {
        latestInfo = item.editor.getRenderInfo(item.pageIndex);
      } on EditorException {
        item.completer.complete(false);
        continue;
      }
      final stableSize =
          latestInfo != null &&
          latestInfo.width > 0 &&
          latestInfo.height > 0 &&
          latestInfo.width == item.width &&
          latestInfo.height == item.height;

      if (!stableSize) {
        item.completer.complete(false);
        continue;
      }

      submitted.add(item);
      payloadItems.add(<String, dynamic>{
        'textureId': item.textureId,
        'editorPtr': item.editorPtr,
        'pageIndex': item.pageIndex,
        'width': item.width,
        'height': item.height,
      });
    }

    if (submitted.isEmpty) {
      return;
    }

    final editorPtrs = submitted.map((item) => item.editorPtr).toSet();
    EditorRenderCoordinator.markBatchStarted(editorPtrs);
    try {
      final response = await _channel.invokeMethod<dynamic>('render', {'items': payloadItems});

      if (response is List && response.length == submitted.length) {
        for (var i = 0; i < submitted.length; i++) {
          submitted[i].completer.complete(response[i] == true);
        }
        return;
      }

      if (response == true) {
        for (final item in submitted) {
          item.completer.complete(true);
        }
        return;
      }

      for (final item in submitted) {
        item.completer.complete(false);
      }
    } catch (_) {
      for (final item in submitted) {
        item.completer.complete(false);
      }
    } finally {
      EditorRenderCoordinator.markBatchFinished(editorPtrs);
    }
  }

  Future<void> dispose() async {
    final id = _textureId;
    if (id == null) {
      return;
    }

    _textureId = null;
    _latestRequestTokenByTexture.remove(id);
    await _channel.invokeMethod<void>('dispose', {'textureId': id});
  }
}
