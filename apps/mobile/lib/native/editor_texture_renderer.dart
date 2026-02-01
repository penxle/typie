import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';

class EditorTextureRenderer {
  EditorTextureRenderer({required this.editor});

  static const _channel = MethodChannel('co.typie.editor_texture');

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

    final result = await _channel.invokeMethod<bool>('render', {
      'textureId': _textureId,
      'editorPtr': editor.handle.address,
      'pageIndex': pageIndex,
      'width': info.width,
      'height': info.height,
    });
    return result ?? false;
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
