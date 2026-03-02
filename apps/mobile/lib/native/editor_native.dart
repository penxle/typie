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

Map<String, int> getSlateOffsets() {
  _ensureInitialized();

  final ptr = _bindings.editor_get_slate_offsets();
  if (ptr == nullptr) {
    throw const EditorException('Failed to get slate offsets');
  }

  final json = ptr.cast<Utf8>().toDartString();
  _bindings.editor_free_string(ptr);

  final list = jsonDecode(json) as List;
  return {for (final item in list) (item as List)[0] as String: item[1] as int};
}

bool validateRegex(String pattern) {
  _ensureInitialized();

  final patternPtr = pattern.toNativeUtf8();
  final result = _bindings.editor_validate_regex(patternPtr.cast());
  calloc.free(patternPtr);

  return result == 1;
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

  void addFontBase(String family, int weight, Uint8List data) {
    _checkDisposed();

    final familyPtr = family.toNativeUtf8();
    final dataPtr = _bindings.editor_alloc(data.length);
    dataPtr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_add_font_base(_handle, familyPtr.cast(), weight, dataPtr, data.length);

    calloc.free(familyPtr);
    _bindings.editor_free(dataPtr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to add font base');
    }
  }

  void addFontChunk(String family, int weight, Uint8List data) {
    _checkDisposed();

    final familyPtr = family.toNativeUtf8();
    final dataPtr = _bindings.editor_alloc(data.length);
    dataPtr.asTypedList(data.length).setAll(0, data);

    final result = _bindings.editor_application_add_font_chunk(_handle, familyPtr.cast(), weight, dataPtr, data.length);

    calloc.free(familyPtr);
    _bindings.editor_free(dataPtr, data.length, data.length);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to add font chunk');
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

  void setFallbackFonts(List<String> names) {
    _checkDisposed();

    final json = jsonEncode(names);
    final jsonPtr = json.toNativeUtf8();

    final result = _bindings.editor_application_set_fallback_fonts(_handle, jsonPtr.cast());

    calloc.free(jsonPtr);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to set fallback fonts');
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

abstract class DocExportMode {
  static const snapshot = 0;
  static const version = 1;
  static const allUpdates = 2;
  static const updatesFrom = 3;
}

final class NativeEditor {
  NativeEditor._(this._handle) : _testConfig = null;

  @visibleForTesting
  NativeEditor.test({bool selectionHit = false, bool interactiveHit = false, Map<String, dynamic>? clipboardData})
    : _handle = nullptr,
      _testConfig = _NativeEditorTestConfig(
        selectionHit: selectionHit,
        interactiveHit: interactiveHit,
        clipboardData: clipboardData,
      );

  final Pointer<EditorHandle> _handle;
  final _NativeEditorTestConfig? _testConfig;
  bool _disposed = false;
  bool _awake = false;

  VoidCallback? onWakeUp;

  bool get isDisposed => _disposed;
  bool get awake => _awake;
  bool get isTest => _testConfig != null;

  void _wakeUp() {
    if (!_awake) {
      _awake = true;
      onWakeUp?.call();
    }
  }

  void resetAwake() {
    _awake = false;
  }

  void dispatch(Map<String, dynamic> message) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }
    _wakeUp();

    final json = jsonEncode(message);
    final jsonPtr = json.toNativeUtf8();

    final result = _bindings.editor_dispatch(_handle, jsonPtr.cast());

    calloc.free(jsonPtr);

    if (result != 0) {
      throw EditorException(_getLastError() ?? 'Failed to dispatch message');
    }
  }

  int tick() {
    _checkDisposed();
    if (isTest) {
      return 0;
    }

    final result = _bindings.editor_tick(_handle);
    if (result != 0) {
      final error = _getLastError();
      if (error != null) {
        throw EditorException(error);
      }
    }
    return result;
  }

  Pointer<Uint8> getSlatePtr() {
    _checkDisposed();
    if (isTest) {
      return nullptr;
    }
    return _bindings.editor_get_slate_ptr(_handle);
  }

  int getSlateLen() {
    _checkDisposed();
    if (isTest) {
      return 0;
    }
    return _bindings.editor_get_slate_len(_handle);
  }

  Pointer<Uint8> getSlabPtr() {
    _checkDisposed();
    if (isTest) {
      return nullptr;
    }
    return _bindings.editor_get_slab_ptr(_handle);
  }

  int getSlabLen() {
    _checkDisposed();
    if (isTest) {
      return 0;
    }
    return _bindings.editor_get_slab_len(_handle);
  }

  void flush() {
    _checkDisposed();
    if (isTest) {
      return;
    }
    _bindings.editor_flush(_handle);
  }

  int getPageCount() {
    _checkDisposed();
    if (isTest) {
      return 0;
    }
    return _bindings.editor_get_page_count(_handle);
  }

  NativeEditorRenderInfo? getRenderInfo(int pageIndex) {
    _checkDisposed();
    if (isTest) {
      return null;
    }

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

  Uint8List? export(int mode, [Uint8List? version]) {
    _checkDisposed();
    if (isTest) {
      return null;
    }

    final Pointer<Uint8> versionPtr;
    final int versionLen;
    if (version != null) {
      final allocated = _allocNative(version);
      versionPtr = allocated.$1;
      versionLen = allocated.$2;
    } else {
      versionPtr = nullptr;
      versionLen = 0;
    }

    return _extractBytes((outLen) {
      final ptr = _bindings.editor_export(_handle, mode, versionPtr, versionLen, outLen);
      if (version != null) {
        _freeNative(versionPtr, versionLen);
      }
      return ptr;
    });
  }

  void importUpdates(Uint8List updates) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }
    _withNativeBytes(updates, (ptr, len) {
      final result = _bindings.editor_import_updates(_handle, ptr, len);
      if (result != 0) {
        throw EditorException(_getLastError() ?? 'Failed to import updates');
      }
    });
    _wakeUp();
  }

  void insertTemplateFragment(Uint8List snapshot) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }
    _withNativeBytes(snapshot, (ptr, len) {
      final result = _bindings.editor_insert_template_fragment(_handle, ptr, len);
      if (result != 0) {
        throw EditorException(_getLastError() ?? 'Failed to insert template fragment');
      }
    });
    _wakeUp();
  }

  void importUpdatesBatch(List<Uint8List> updatesBatch) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }

    if (updatesBatch.isEmpty) {
      return;
    }

    final count = updatesBatch.length;
    final ptrsArray = calloc<Pointer<Uint8>>(count);
    final lensArray = calloc<Size>(count);

    try {
      for (var i = 0; i < count; i++) {
        final (ptr, len) = _allocNative(updatesBatch[i]);
        ptrsArray[i] = ptr;
        lensArray[i] = len;
      }

      final result = _bindings.editor_import_updates_batch(_handle, ptrsArray, lensArray, count);

      for (var i = 0; i < count; i++) {
        _freeNative(ptrsArray[i], lensArray[i]);
      }

      if (result != 0) {
        throw EditorException(_getLastError() ?? 'Failed to import updates batch');
      }
    } finally {
      calloc
        ..free(ptrsArray)
        ..free(lensArray);
    }
    _wakeUp();
  }

  Map<String, dynamic>? getClipboardData() {
    _checkDisposed();
    if (isTest) {
      return _testConfig!.clipboardData;
    }

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

  bool isSelectionHit(int pageIdx, double x, double y) {
    _checkDisposed();
    if (isTest) {
      return _testConfig!.selectionHit;
    }
    return _bindings.editor_is_selection_hit(_handle, pageIdx, x, y) == 1;
  }

  bool isInteractiveHit(int pageIdx, double x, double y) {
    _checkDisposed();
    if (isTest) {
      return _testConfig!.interactiveHit;
    }
    return _bindings.editor_is_interactive_hit(_handle, pageIdx, x, y) == 1;
  }

  NativeEditorCharacterCounts getCharacterCounts() {
    _checkDisposed();
    if (isTest) {
      return const NativeEditorCharacterCounts(
        docWithWhitespace: 0,
        docWithoutWhitespace: 0,
        docWithoutWhitespaceAndPunctuation: 0,
        selectionWithWhitespace: 0,
        selectionWithoutWhitespace: 0,
        selectionWithoutWhitespaceAndPunctuation: 0,
      );
    }

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

  Map<String, dynamic>? getTextWithMappings() {
    _checkDisposed();
    if (isTest) {
      return null;
    }

    final ptr = _bindings.editor_get_text_with_mappings(_handle);
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

  void setTrackedItems(int group, List<Map<String, dynamic>> items) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }

    final json = jsonEncode(items);
    final jsonPtr = json.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_set_tracked_items(_handle, group, jsonPtr);
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to set tracked items');
      }
    } finally {
      calloc.free(jsonPtr);
    }
    _wakeUp();
  }

  void removeTrackedItems(int group, List<String> ids) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }

    final json = jsonEncode(ids);
    final jsonPtr = json.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_remove_tracked_items(_handle, group, jsonPtr);
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to remove tracked items');
      }
    } finally {
      calloc.free(jsonPtr);
    }
    _wakeUp();
  }

  List<Map<String, dynamic>> performSearch(String query, bool matchWholeWord) {
    _checkDisposed();
    if (isTest) {
      return [];
    }

    final queryPtr = query.toNativeUtf8().cast<Char>();

    try {
      final ptr = _bindings.editor_perform_search(_handle, queryPtr, matchWholeWord ? 1 : 0);
      if (ptr == nullptr) {
        final error = _getLastError();
        if (error != null) {
          throw EditorException(error);
        }
        return [];
      }

      final json = ptr.cast<Utf8>().toDartString();
      _bindings.editor_free_string(ptr);

      return (jsonDecode(json) as List<dynamic>).cast<Map<String, dynamic>>();
    } finally {
      calloc.free(queryPtr);
    }
  }

  bool replaceTextInBlock(String blockId, int startOffset, int endOffset, String replacement) {
    _checkDisposed();
    if (isTest) {
      return false;
    }

    final blockIdPtr = blockId.toNativeUtf8().cast<Char>();
    final replacementPtr = replacement.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_replace_text_in_block(
        _handle,
        blockIdPtr,
        startOffset,
        endOffset,
        replacementPtr,
      );
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to replace text in block');
      }
      _wakeUp();
      return result == 1;
    } finally {
      calloc
        ..free(blockIdPtr)
        ..free(replacementPtr);
    }
  }

  void replaceTextInBlocks(List<List<dynamic>> items) {
    _checkDisposed();
    if (isTest) {
      _wakeUp();
      return;
    }

    final json = jsonEncode(items);
    final jsonPtr = json.toNativeUtf8().cast<Char>();

    try {
      final result = _bindings.editor_replace_text_in_blocks(_handle, jsonPtr);
      if (result < 0) {
        throw EditorException(_getLastError() ?? 'Failed to replace text in blocks');
      }
      _wakeUp();
    } finally {
      calloc.free(jsonPtr);
    }
  }

  NativeDragImageResult? renderDragImage(List<int> visiblePages, int pageIdx) {
    _checkDisposed();
    if (isTest) {
      return null;
    }

    final visiblePagesPtr = _bindings.editor_alloc(visiblePages.length * sizeOf<Size>());
    final visiblePagesTyped = visiblePagesPtr.cast<Size>();
    for (var i = 0; i < visiblePages.length; i++) {
      visiblePagesTyped[i] = visiblePages[i];
    }

    final resultPtr = calloc<DragImageResult>();

    try {
      final status = _bindings.editor_render_drag_image(
        _handle,
        visiblePagesPtr.cast(),
        visiblePages.length,
        pageIdx,
        resultPtr,
      );

      if (status != 0) {
        throw EditorException(_getLastError() ?? 'Failed to render drag image');
      }

      final result = resultPtr.ref;
      final len = result.len;
      final pixels = Uint8List.fromList(result.pixels.asTypedList(len));

      _bindings.editor_free(result.pixels, len, len);

      return NativeDragImageResult(
        width: result.width,
        height: result.height,
        offsetX: result.offset_x,
        offsetY: result.offset_y,
        scaleFactor: result.scale_factor,
        pixels: pixels,
      );
    } finally {
      _bindings.editor_free(
        visiblePagesPtr,
        visiblePages.length * sizeOf<Size>(),
        visiblePages.length * sizeOf<Size>(),
      );
      calloc.free(resultPtr);
    }
  }

  (Pointer<Uint8>, int) _allocNative(Uint8List data) {
    final ptr = _bindings.editor_alloc(data.length);
    ptr.asTypedList(data.length).setAll(0, data);
    return (ptr, data.length);
  }

  void _freeNative(Pointer<Uint8> ptr, int len) {
    _bindings.editor_free(ptr, len, len);
  }

  Uint8List? _extractBytes(Pointer<Uint8> Function(Pointer<Size> outLen) fn) {
    _checkDisposed();

    final outLen = calloc<Size>();
    try {
      final ptr = fn(outLen);
      if (ptr == nullptr) {
        return null;
      }

      final len = outLen.value;
      final data = Uint8List.fromList(ptr.asTypedList(len));
      _freeNative(ptr, len);
      return data;
    } finally {
      calloc.free(outLen);
    }
  }

  void _withNativeBytes(Uint8List data, void Function(Pointer<Uint8> ptr, int len) fn) {
    final (ptr, len) = _allocNative(data);
    try {
      fn(ptr, len);
    } finally {
      _freeNative(ptr, len);
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

final class _NativeEditorTestConfig {
  const _NativeEditorTestConfig({
    required this.selectionHit,
    required this.interactiveHit,
    required this.clipboardData,
  });

  final bool selectionHit;
  final bool interactiveHit;
  final Map<String, dynamic>? clipboardData;
}

final class NativeDragImageResult {
  const NativeDragImageResult({
    required this.width,
    required this.height,
    required this.offsetX,
    required this.offsetY,
    required this.scaleFactor,
    required this.pixels,
  });

  final int width;
  final int height;
  final double offsetX;
  final double offsetY;
  final double scaleFactor;
  final Uint8List pixels;
}
