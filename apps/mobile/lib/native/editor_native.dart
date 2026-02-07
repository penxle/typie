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

  void addFont(String name, int weight, Uint8List data) {
    _checkDisposed();

    final namePtr = name.toNativeUtf8();
    final dataPtr = _bindings.editor_alloc(data.length);
    dataPtr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_add_font(_handle, namePtr.cast(), weight, dataPtr, data.length);

    calloc.free(namePtr);
    _bindings.editor_free(dataPtr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to add font');
    }
  }

  void registerFallbackFont(String name) {
    _checkDisposed();

    final namePtr = name.toNativeUtf8();
    final result = _bindings.editor_application_register_fallback_font(_handle, namePtr.cast());
    calloc.free(namePtr);

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

  void setTextReplacementRules(List<Map<String, dynamic>> rules) {
    _checkDisposed();

    final json = jsonEncode(rules);
    final jsonPtr = json.toNativeUtf8();

    final result = _bindings.editor_application_set_text_replacement_rules(_handle, jsonPtr.cast());

    calloc.free(jsonPtr);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to set text replacement rules');
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

final class NativeEditorRenderInfo {
  const NativeEditorRenderInfo({required this.width, required this.height, required this.bufferSize});

  final int width;
  final int height;
  final int bufferSize;
}

final class NativeEditorCharacterCounts {
  const NativeEditorCharacterCounts({
    required this.docWithWhitespace,
    required this.docWithoutWhitespace,
    required this.docWithoutWhitespaceAndPunctuation,
    required this.selectionWithWhitespace,
    required this.selectionWithoutWhitespace,
    required this.selectionWithoutWhitespaceAndPunctuation,
  });

  final int docWithWhitespace;
  final int docWithoutWhitespace;
  final int docWithoutWhitespaceAndPunctuation;
  final int selectionWithWhitespace;
  final int selectionWithoutWhitespace;
  final int selectionWithoutWhitespaceAndPunctuation;
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

  NativeEditorRenderInfo? getRenderInfo(int pageIndex) {
    _checkDisposed();

    final infoPtr = calloc<RenderInfo>();
    try {
      final status = _bindings.editor_get_render_info(_handle, pageIndex, infoPtr);
      if (status != 0) {
        return null;
      }

      return NativeEditorRenderInfo(
        width: infoPtr.ref.width,
        height: infoPtr.ref.height,
        bufferSize: infoPtr.ref.buffer_size,
      );
    } finally {
      calloc.free(infoPtr);
    }
  }

  Pointer<EditorHandle> get handle => _handle;

  Uint8List? getSnapshot() {
    _checkDisposed();

    final outLen = calloc<Size>();
    try {
      final ptr = _bindings.editor_get_snapshot(_handle, outLen);
      if (ptr == nullptr) {
        return null;
      }

      final len = outLen.value;
      final data = Uint8List.fromList(ptr.asTypedList(len));
      _bindings.editor_free(ptr, len, len);
      return data;
    } finally {
      calloc.free(outLen);
    }
  }

  Uint8List? getVersion() {
    _checkDisposed();

    final outLen = calloc<Size>();
    try {
      final ptr = _bindings.editor_get_version(_handle, outLen);
      if (ptr == nullptr) {
        return null;
      }

      final len = outLen.value;
      final data = Uint8List.fromList(ptr.asTypedList(len));
      _bindings.editor_free(ptr, len, len);
      return data;
    } finally {
      calloc.free(outLen);
    }
  }

  Uint8List? exportAllUpdates() {
    _checkDisposed();

    final outLen = calloc<Size>();
    try {
      final ptr = _bindings.editor_export_all_updates(_handle, outLen);
      if (ptr == nullptr) {
        return null;
      }

      final len = outLen.value;
      final data = Uint8List.fromList(ptr.asTypedList(len));
      _bindings.editor_free(ptr, len, len);
      return data;
    } finally {
      calloc.free(outLen);
    }
  }

  ({Uint8List updates, Uint8List version})? exportNewUpdates() {
    _checkDisposed();

    final outUpdates = calloc<Pointer<Uint8>>();
    final outUpdatesLen = calloc<Size>();
    final outVersion = calloc<Pointer<Uint8>>();
    final outVersionLen = calloc<Size>();

    try {
      final result = _bindings.editor_export_new_updates(_handle, outUpdates, outUpdatesLen, outVersion, outVersionLen);

      if (result != 0) {
        return null;
      }

      final updatesPtr = outUpdates.value;
      final updatesLen = outUpdatesLen.value;
      final versionPtr = outVersion.value;
      final versionLen = outVersionLen.value;

      final updates = Uint8List.fromList(updatesPtr.asTypedList(updatesLen));
      final version = Uint8List.fromList(versionPtr.asTypedList(versionLen));

      _bindings
        ..editor_free(updatesPtr, updatesLen, updatesLen)
        ..editor_free(versionPtr, versionLen, versionLen);

      return (updates: updates, version: version);
    } finally {
      calloc
        ..free(outUpdates)
        ..free(outUpdatesLen)
        ..free(outVersion)
        ..free(outVersionLen);
    }
  }

  void importUpdates(Uint8List updates) {
    _checkDisposed();

    final ptr = _bindings.editor_alloc(updates.length);
    ptr.asTypedList(updates.length).setAll(0, updates);

    final result = _bindings.editor_import_updates(_handle, ptr, updates.length);
    _bindings.editor_free(ptr, updates.length, updates.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to import updates');
    }
  }

  void importUpdatesBatch(List<Uint8List> updatesBatch) {
    _checkDisposed();

    if (updatesBatch.isEmpty) {
      return;
    }

    final count = updatesBatch.length;
    final ptrsArray = calloc<Pointer<Uint8>>(count);
    final lensArray = calloc<Size>(count);

    try {
      for (var i = 0; i < count; i++) {
        final update = updatesBatch[i];
        final ptr = _bindings.editor_alloc(update.length);
        ptr.asTypedList(update.length).setAll(0, update);
        ptrsArray[i] = ptr;
        lensArray[i] = update.length;
      }

      final result = _bindings.editor_import_updates_batch(_handle, ptrsArray, lensArray, count);

      for (var i = 0; i < count; i++) {
        _bindings.editor_free(ptrsArray[i], lensArray[i], lensArray[i]);
      }

      if (result != 0) {
        throw EditorException(_getLastError() ?? 'Failed to import updates batch');
      }
    } finally {
      calloc
        ..free(ptrsArray)
        ..free(lensArray);
    }
  }

  void commitSync(Uint8List version) {
    _checkDisposed();

    final ptr = _bindings.editor_alloc(version.length);
    ptr.asTypedList(version.length).setAll(0, version);

    final result = _bindings.editor_commit_sync(_handle, ptr, version.length);
    _bindings.editor_free(ptr, version.length, version.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to commit sync');
    }
  }

  Map<String, dynamic>? getClipboardData() {
    _checkDisposed();

    final ptr = _bindings.editor_get_clipboard_data(_handle);
    if (ptr == nullptr) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
      return null;
    }

    final json = ptr.cast<Utf8>().toDartString();
    _bindings.editor_free_string(ptr);

    return jsonDecode(json) as Map<String, dynamic>;
  }

  NativeEditorCharacterCounts getCharacterCounts() {
    _checkDisposed();

    final countsPtr = calloc<CharacterCounts>();
    try {
      final result = _bindings.editor_get_character_counts(_handle, countsPtr);
      if (result != 0) {
        throw EditorException(_getLastError() ?? 'Failed to get character counts');
      }

      return NativeEditorCharacterCounts(
        docWithWhitespace: countsPtr.ref.doc_with_whitespace,
        docWithoutWhitespace: countsPtr.ref.doc_without_whitespace,
        docWithoutWhitespaceAndPunctuation: countsPtr.ref.doc_without_whitespace_and_punctuation,
        selectionWithWhitespace: countsPtr.ref.selection_with_whitespace,
        selectionWithoutWhitespace: countsPtr.ref.selection_without_whitespace,
        selectionWithoutWhitespaceAndPunctuation: countsPtr.ref.selection_without_whitespace_and_punctuation,
      );
    } finally {
      calloc.free(countsPtr);
    }
  }

  Map<String, dynamic>? getSpellcheckText() {
    _checkDisposed();

    final ptr = _bindings.editor_get_spellcheck_text(_handle);
    if (ptr == nullptr) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
      return null;
    }

    final json = ptr.cast<Utf8>().toDartString();
    _bindings.editor_free_string(ptr);

    return jsonDecode(json) as Map<String, dynamic>;
  }

  void setSpellcheckErrors(List<Map<String, dynamic>> errors) {
    _checkDisposed();

    final json = jsonEncode(errors);
    final jsonPtr = json.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_set_spellcheck_errors(_handle, jsonPtr);
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to set spellcheck errors');
      }
    } finally {
      calloc.free(jsonPtr);
    }
  }

  bool applySpellcheckCorrection(String blockId, int startOffset, int endOffset, String correction) {
    _checkDisposed();

    final blockIdPtr = blockId.toNativeUtf8().cast<Char>();
    final correctionPtr = correction.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_apply_spellcheck_correction(
        _handle,
        blockIdPtr,
        startOffset,
        endOffset,
        correctionPtr,
      );
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to apply spellcheck correction');
      }
      return result == 1;
    } finally {
      calloc
        ..free(blockIdPtr)
        ..free(correctionPtr);
    }
  }

  List<dynamic> getSpellcheckErrors() {
    _checkDisposed();

    final ptr = _bindings.editor_get_spellcheck_errors(_handle);
    if (ptr == nullptr) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
      return [];
    }

    final json = ptr.cast<Utf8>().toDartString();
    _bindings.editor_free_string(ptr);

    return jsonDecode(json) as List<dynamic>;
  }

  void clearSpellcheckErrors() {
    _checkDisposed();

    final result = _bindings.editor_clear_spellcheck_errors(_handle);
    if (result < 0) {
      throw EditorException(_getLastError() ?? 'Failed to clear spellcheck errors');
    }
  }

  void setAiFeedbackItems(List<Map<String, dynamic>> items) {
    _checkDisposed();

    final json = jsonEncode(items);
    final jsonPtr = json.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_set_ai_feedback_items(_handle, jsonPtr);
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to set ai feedback items');
      }
    } finally {
      calloc.free(jsonPtr);
    }
  }

  void clearAiFeedbackItems() {
    _checkDisposed();

    final result = _bindings.editor_clear_ai_feedback_items(_handle);
    if (result < 0) {
      throw EditorException(_getLastError() ?? 'Failed to clear ai feedback items');
    }
  }

  List<dynamic> getAiFeedbackItems() {
    _checkDisposed();

    final ptr = _bindings.editor_get_ai_feedback_items(_handle);
    if (ptr == nullptr) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
      return [];
    }

    final json = ptr.cast<Utf8>().toDartString();
    _bindings.editor_free_string(ptr);

    return jsonDecode(json) as List<dynamic>;
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
