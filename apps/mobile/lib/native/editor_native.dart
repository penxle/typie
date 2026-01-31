import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';

import 'editor_bindings.dart';

const _logLevelDebug = 0;
const _logLevelInfo = 1;
const _logLevelWarn = 2;
const _logLevelError = 3;

typedef _LogCallbackFunc = Void Function(Int32 level, Pointer<Char> message);

@pragma('vm:entry-point')
void _nativeLogCallback(int level, Pointer<Char> messagePtr) {
  final message = messagePtr.cast<Utf8>().toDartString();
  final prefix = switch (level) {
    _logLevelDebug => '[DEBUG]',
    _logLevelInfo => '[INFO]',
    _logLevelWarn => '[WARN]',
    _logLevelError => '[ERROR]',
    _ => '[UNKNOWN]',
  };
  debugPrint('$prefix $message');
}

late EditorBindings _bindings;
bool _initialized = false;

void _ensureInitialized() {
  if (_initialized) {
    return;
  }
  _initialized = true;

  final DynamicLibrary dylib;
  if (Platform.isAndroid) {
    dylib = DynamicLibrary.open('libeditor.so');
  } else if (Platform.isIOS) {
    dylib = DynamicLibrary.process();
  } else {
    throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
  }

  _bindings = EditorBindings(dylib);

  final callbackPtr = Pointer.fromFunction<_LogCallbackFunc>(_nativeLogCallback);
  _bindings.editor_set_log_callback(callbackPtr);
}

final class EditorException implements Exception {
  const EditorException(this.message);

  final String message;

  @override
  String toString() => 'EditorException: $message';
}

String? _getLastError() {
  final ptr = _bindings.editor_get_last_error();
  if (ptr == nullptr) {
    return null;
  }

  final error = ptr.cast<Utf8>().toDartString();
  _bindings
    ..editor_free_string(ptr)
    ..editor_clear_last_error();
  return error;
}

class NativeEditorApplication {
  NativeEditorApplication() : _handle = _createHandle();

  static Pointer<EditorApplication> _createHandle() {
    _ensureInitialized();
    final handle = _bindings.editor_application_new();
    if (handle == nullptr) {
      throw EditorException(_getLastError() ?? 'Failed to create EditorApplication');
    }
    return handle;
  }

  final Pointer<EditorApplication> _handle;
  bool _disposed = false;

  void loadIcuData(Uint8List data) {
    _checkDisposed();

    final ptr = _bindings.editor_alloc(data.length);
    ptr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_load_icu_data(_handle, ptr, data.length);
    _bindings.editor_free(ptr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to load ICU data');
    }
  }

  void registerFont(String name, int weight, Uint8List data) {
    _checkDisposed();

    final namePtr = name.toNativeUtf8();
    final dataPtr = _bindings.editor_alloc(data.length);
    dataPtr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_register_font(_handle, namePtr.cast(), weight, dataPtr, data.length);

    calloc.free(namePtr);
    _bindings.editor_free(dataPtr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to register font');
    }
  }

  void registerFallbackFont(String name, int weight, Uint8List data) {
    _checkDisposed();

    final namePtr = name.toNativeUtf8();
    final dataPtr = _bindings.editor_alloc(data.length);
    dataPtr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_register_fallback_font(
      _handle,
      namePtr.cast(),
      weight,
      dataPtr,
      data.length,
    );

    calloc.free(namePtr);
    _bindings.editor_free(dataPtr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to register fallback font');
    }
  }

  void setAvailableFonts(Map<String, List<int>> fonts) {
    _checkDisposed();

    final json = jsonEncode(fonts);
    final jsonPtr = json.toNativeUtf8();

    final result = _bindings.editor_application_set_available_fonts(_handle, jsonPtr.cast());

    calloc.free(jsonPtr);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to set available fonts');
    }
  }

  NativeEditor createEditor(double scaleFactor, {Uint8List? snapshot}) {
    _checkDisposed();

    final snapshotLen = snapshot?.length ?? 0;
    final snapshotPtr = snapshot != null ? _bindings.editor_alloc(snapshotLen) : nullptr;

    if (snapshot != null) {
      snapshotPtr.asTypedList(snapshotLen).setAll(0, snapshot);
    }

    final editorHandle = _bindings.editor_application_create_editor(_handle, scaleFactor, snapshotPtr, snapshotLen);

    if (snapshot != null) {
      _bindings.editor_free(snapshotPtr, snapshotLen, snapshotLen);
    }

    if (editorHandle == nullptr) {
      throw EditorException(_getLastError() ?? 'Failed to create Editor');
    }

    return NativeEditor._(editorHandle);
  }

  void dispose() {
    if (!_disposed) {
      _bindings.editor_application_free(_handle);
      _disposed = true;
    }
  }

  void _checkDisposed() {
    if (_disposed) {
      throw const EditorException('EditorApplication has been disposed');
    }
  }
}

final class NativeEditorRenderResult {
  const NativeEditorRenderResult({required this.data, required this.width, required this.height});

  final Uint8List data;
  final int width;
  final int height;
}

final class NativeEditor {
  NativeEditor._(this._handle);

  final Pointer<EditorHandle> _handle;
  bool _disposed = false;

  bool get isDisposed => _disposed;

  void dispatch(Map<String, dynamic> message) {
    _checkDisposed();

    final json = jsonEncode(message);
    final jsonPtr = json.toNativeUtf8();

    final result = _bindings.editor_dispatch(_handle, jsonPtr.cast());

    calloc.free(jsonPtr);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to dispatch message');
    }
  }

  List<dynamic>? tick() {
    _checkDisposed();

    final ptr = _bindings.editor_tick(_handle);
    if (ptr == nullptr) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
      return null;
    }

    final json = ptr.cast<Utf8>().toDartString();
    _bindings.editor_free_string(ptr);

    final result = jsonDecode(json) as List<dynamic>;
    return result.isEmpty ? null : result;
  }

  void flush() {
    _checkDisposed();
    _bindings.editor_flush(_handle);
  }

  int getPageCount() {
    _checkDisposed();
    return _bindings.editor_get_page_count(_handle);
  }

  NativeEditorRenderResult renderPage(int pageIndex) {
    _checkDisposed();

    final resultPtr = calloc<RenderResult>();
    try {
      final status = _bindings.editor_render_page(_handle, pageIndex, resultPtr);
      if (status != 0) {
        throw EditorException(_getLastError() ?? 'Failed to render page $pageIndex');
      }

      return NativeEditorRenderResult(
        data: Uint8List.fromList(resultPtr.ref.ptr.asTypedList(resultPtr.ref.len)),
        width: resultPtr.ref.width,
        height: resultPtr.ref.height,
      );
    } finally {
      calloc.free(resultPtr);
    }
  }

  void dispose() {
    if (!_disposed) {
      _bindings.editor_handle_free(_handle);
      _disposed = true;
    }
  }

  void _checkDisposed() {
    if (_disposed) {
      throw const EditorException('Editor has been disposed');
    }
  }
}
