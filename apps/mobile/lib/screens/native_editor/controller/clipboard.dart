import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:mime/mime.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:uuid/uuid.dart';

class EditorClipboard {
  static const _channel = MethodChannel('co.typie.clipboard');
  static const _uuid = Uuid();
  static final List<FileFormat> _fileFormats = [
    for (final format in Formats.standardFormats)
      if (format is FileFormat) format,
  ];

  Future<void> copy(NativeEditor editor) async {
    final data = editor.getClipboardData();
    if (data == null) {
      return;
    }
    await _channel.invokeMethod('setData', {'text': data['text'] as String, 'html': data['html'] as String});
  }

  Future<void> cut(NativeEditor editor, void Function(Map<String, dynamic>) dispatch) async {
    await copy(editor);
    dispatch({'type': 'deleteSelection'});
  }

  Future<bool> handlePaste({
    required UploadManager uploadManager,
    required void Function(Map<String, dynamic>) dispatch,
    required void Function() scrollIntoView,
    required bool restrictedBlob,
    void Function(String reason)? onEditBlocked,
  }) async {
    final payload = await getPastePayload();
    final reader = await _readClipboardReader();
    if (reader != null && _hasClipboardAssets(reader)) {
      final insertions = await _readClipboardInsertions(reader);
      if (insertions.isNotEmpty) {
        if (_shouldPreferPayloadOverInsertions(payload, insertions)) {
          dispatch(payload!);
          scrollIntoView();
          return true;
        }

        if (restrictedBlob) {
          onEditBlocked?.call('restrictedBlob');
          return true;
        }

        _insertClipboardInsertions(insertions, uploadManager: uploadManager, dispatch: dispatch);
        scrollIntoView();
        return true;
      }
    }

    if (payload == null) {
      return false;
    }

    dispatch(payload);
    scrollIntoView();
    return true;
  }

  Future<bool> handleInsertedContent({
    required KeyboardInsertedContent content,
    required UploadManager uploadManager,
    required void Function(Map<String, dynamic>) dispatch,
    required void Function() scrollIntoView,
    required bool restrictedBlob,
    void Function(String reason)? onEditBlocked,
  }) async {
    if (_canInsertContent(content)) {
      if (restrictedBlob) {
        onEditBlocked?.call('restrictedBlob');
        return true;
      }

      final insertion = await _readInsertedContent(content);
      if (insertion != null) {
        _insertClipboardInsertions([insertion], uploadManager: uploadManager, dispatch: dispatch);
        scrollIntoView();
        return true;
      }
    }

    return false;
  }

  Future<Map<String, dynamic>?> getPastePayload() async {
    final data = await _channel.invokeMapMethod<String, String?>('getData') ?? {};
    final text = data['text'] ?? '';
    final html = data['html'];

    if (html != null) {
      return {'type': 'pasteHtml', 'html': html, 'text': text};
    } else if (text.isNotEmpty) {
      return {'type': 'pasteText', 'text': text};
    }
    return null;
  }

  Future<ClipboardReader?> _readClipboardReader() async {
    final clipboard = SystemClipboard.instance;
    if (clipboard == null) {
      return null;
    }

    try {
      return await clipboard.read();
    } catch (_) {
      return null;
    }
  }

  bool _hasClipboardAssets(ClipboardReader reader) {
    for (final item in reader.items) {
      if (item.canProvide(Formats.fileUri)) {
        return true;
      }
      if (_firstAvailableFormat(item, where: _isAssetFileFormat) != null) {
        return true;
      }
    }
    return false;
  }

  Future<List<_ClipboardInsertion>> _readClipboardInsertions(ClipboardReader reader) async {
    final insertions = <_ClipboardInsertion>[];
    for (final item in reader.items) {
      final insertion = await _readClipboardItem(item);
      if (insertion != null) {
        insertions.add(insertion);
      }
    }
    return insertions;
  }

  void _insertClipboardInsertions(
    List<_ClipboardInsertion> insertions, {
    required UploadManager uploadManager,
    required void Function(Map<String, dynamic>) dispatch,
  }) {
    for (final insertion in insertions) {
      insertion.insert(uploadManager: uploadManager, dispatch: dispatch);
    }
  }

  Future<_ClipboardInsertion?> _readClipboardItem(ClipboardDataReader item) async {
    final fileUriInsertion = await _readClipboardFileUriItem(item);
    if (fileUriInsertion != null) {
      return fileUriInsertion;
    }

    final imageFormat = _firstAvailableFormat(item, where: _isImageFormat);
    if (imageFormat != null) {
      final imageInsertion = await _readClipboardFormatItem(item, imageFormat);
      if (imageInsertion != null) {
        return imageInsertion;
      }
    }

    final fileFormat = _firstAvailableFormat(
      item,
      where: (format) => _isAssetFileFormat(format) && !_isImageFormat(format),
    );
    if (fileFormat != null) {
      return _readClipboardFormatItem(item, fileFormat);
    }

    return null;
  }

  Future<_ClipboardInsertion?> _readClipboardFileUriItem(ClipboardDataReader item) async {
    if (!item.canProvide(Formats.fileUri)) {
      return null;
    }

    final uri = await item.readValue(Formats.fileUri);
    if (uri == null || !uri.isScheme('file')) {
      return null;
    }

    final path = uri.toFilePath();
    if (path.isEmpty) {
      return null;
    }

    return _readFilePathInsertion(path);
  }

  Future<_ClipboardInsertion?> _readClipboardFormatItem(ClipboardDataReader item, FileFormat format) async {
    final file = await _readFile(item, format);
    if (file == null) {
      return null;
    }

    try {
      final bytes = await file.readAll();
      if (_isImageFormat(format)) {
        final imageInfo = await _readImageInfo(bytes);
        if (imageInfo != null) {
          return _ClipboardImageInsertion(
            uploadId: _uuid.v4(),
            name: _sanitizeFileName(
              file.fileName ?? await item.getSuggestedName() ?? _defaultFileNameForFormat(format),
            ),
            bytes: imageInfo.bytes,
            width: imageInfo.width,
            height: imageInfo.height,
          );
        }
      }

      return _ClipboardFileInsertion(
        uploadId: _uuid.v4(),
        name: _sanitizeFileName(file.fileName ?? await item.getSuggestedName() ?? _defaultFileNameForFormat(format)),
        size: bytes.lengthInBytes,
        bytes: bytes,
        mimeType: _preferredMimeTypeForFormat(format),
      );
    } catch (_) {
      return null;
    }
  }

  Future<_ClipboardInsertion?> _readFilePathInsertion(String path) async {
    final file = File(path);
    if (!file.existsSync()) {
      return null;
    }

    final mimeType = await _mimeTypeForFile(file);
    if (mimeType?.startsWith('image/') ?? false) {
      final imageInfo = await _readImageInfo(await file.readAsBytes());
      if (imageInfo != null) {
        return _ClipboardImageInsertion(
          uploadId: _uuid.v4(),
          name: _fileNameFromPath(path),
          bytes: imageInfo.bytes,
          width: imageInfo.width,
          height: imageInfo.height,
        );
      }
    }

    return _ClipboardFileInsertion(
      uploadId: _uuid.v4(),
      path: path,
      name: _fileNameFromPath(path),
      size: file.lengthSync(),
    );
  }

  bool _canInsertContent(KeyboardInsertedContent content) {
    if (content.hasData && !_isTextualMimeType(content.mimeType)) {
      return true;
    }

    final uri = Uri.tryParse(content.uri);
    return uri?.isScheme('file') ?? false;
  }

  Future<_ClipboardInsertion?> _readInsertedContent(KeyboardInsertedContent content) async {
    if (content.hasData) {
      if (_isTextualMimeType(content.mimeType)) {
        return null;
      }

      final bytes = content.data!;
      if (content.mimeType.startsWith('image/')) {
        final imageInfo = await _readImageInfo(bytes);
        if (imageInfo != null) {
          return _ClipboardImageInsertion(
            uploadId: _uuid.v4(),
            name: _sanitizeFileName(
              _suggestedFileNameFromUri(content.uri) ?? _defaultFileNameForMimeType(content.mimeType),
            ),
            bytes: imageInfo.bytes,
            width: imageInfo.width,
            height: imageInfo.height,
          );
        }
      }

      return _ClipboardFileInsertion(
        uploadId: _uuid.v4(),
        name: _sanitizeFileName(
          _suggestedFileNameFromUri(content.uri) ?? _defaultFileNameForMimeType(content.mimeType),
        ),
        size: bytes.lengthInBytes,
        bytes: bytes,
        mimeType: content.mimeType,
      );
    }

    final uri = Uri.tryParse(content.uri);
    if (uri == null || !uri.isScheme('file')) {
      return null;
    }

    return _readFilePathInsertion(uri.toFilePath());
  }

  FileFormat? _firstAvailableFormat(ClipboardDataReader item, {bool Function(FileFormat format)? where}) {
    for (final format in _fileFormats) {
      if (item.canProvide(format) && (where == null || where(format))) {
        return format;
      }
    }
    return null;
  }

  Future<DataReaderFile?> _readFile(DataReader reader, FileFormat format) {
    final completer = Completer<DataReaderFile?>();
    final progress = reader.getFile(
      format,
      (file) {
        if (!completer.isCompleted) {
          completer.complete(file);
        }
      },
      onError: (error) {
        if (!completer.isCompleted) {
          completer.completeError(error);
        }
      },
    );

    if (progress == null && !completer.isCompleted) {
      completer.complete(null);
    }

    return completer.future;
  }

  Future<({Uint8List bytes, int width, int height})?> _readImageInfo(Uint8List bytes) async {
    try {
      final codec = await ui.instantiateImageCodec(bytes);
      try {
        final frame = await codec.getNextFrame();
        try {
          return (bytes: bytes, width: frame.image.width, height: frame.image.height);
        } finally {
          frame.image.dispose();
        }
      } finally {
        codec.dispose();
      }
    } catch (_) {
      return null;
    }
  }

  Future<String?> _mimeTypeForFile(File file) async {
    final headerBytes = <int>[];
    await file.openRead(0, defaultMagicNumbersMaxLength).forEach(headerBytes.addAll);
    return lookupMimeType(file.path, headerBytes: headerBytes);
  }

  String? _preferredMimeTypeForFormat(FileFormat format) {
    if (format is! SimpleFileFormat) {
      return null;
    }

    final mimeTypes = format.mimeTypes;
    if (mimeTypes != null) {
      for (final mimeType in mimeTypes) {
        if (mimeType.isNotEmpty) {
          return mimeType;
        }
      }
    }

    final providerFormat = format.providerFormat;
    return providerFormat.contains('/') ? providerFormat : null;
  }

  String _fileNameFromPath(String path) {
    final segments = File(path).uri.pathSegments;
    if (segments.isNotEmpty && segments.last.isNotEmpty) {
      return segments.last;
    }
    final normalized = path.replaceAll(r'\', '/');
    final idx = normalized.lastIndexOf('/');
    return idx >= 0 ? normalized.substring(idx + 1) : normalized;
  }

  String _sanitizeFileName(String value) {
    final sanitized = value.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
    return sanitized.isEmpty ? 'clipboard-file' : sanitized;
  }

  String? _suggestedFileNameFromUri(String uriValue) {
    final uri = Uri.tryParse(uriValue);
    if (uri == null) {
      return null;
    }

    final segments = uri.pathSegments.where((segment) => segment.isNotEmpty);
    if (segments.isEmpty) {
      return null;
    }

    return segments.last;
  }

  bool _isImageFormat(FileFormat format) {
    if (format is! SimpleFileFormat) {
      return false;
    }

    if (format.mimeTypes?.any((mime) => mime.startsWith('image/')) ?? false) {
      return true;
    }

    if (format.uniformTypeIdentifiers?.any((uti) => uti.contains('image')) ?? false) {
      return true;
    }

    final providerFormat = format.providerFormat;
    return providerFormat.startsWith('image/') || providerFormat.contains('image');
  }

  bool _isAssetFileFormat(FileFormat format) {
    return !_isTextualFileFormat(format);
  }

  bool _isTextualFileFormat(FileFormat format) {
    if (format is! SimpleFileFormat) {
      return false;
    }

    if (format.mimeTypes?.any(_isTextualMimeType) ?? false) {
      return true;
    }

    if (format.uniformTypeIdentifiers?.any(
          (uti) => uti == 'public.utf8-plain-text' || uti == 'public.plain-text' || uti == 'public.html',
        ) ??
        false) {
      return true;
    }

    return _isTextualMimeType(format.providerFormat);
  }

  bool _isTextualMimeType(String mimeType) {
    return mimeType == 'text/plain' || mimeType == 'text/html';
  }

  String _defaultFileNameForFormat(FileFormat format) {
    return _isImageFormat(format) ? 'clipboard-image' : 'clipboard-file';
  }

  String _defaultFileNameForMimeType(String mimeType) {
    return mimeType.startsWith('image/') ? 'clipboard-image' : 'clipboard-file';
  }

  bool _shouldPreferPayloadOverInsertions(Map<String, dynamic>? payload, List<_ClipboardInsertion> insertions) {
    if (payload == null) {
      return false;
    }

    if (payload['type'] != 'pasteHtml') {
      return false;
    }

    return insertions.every((insertion) => insertion.kind == _ClipboardInsertionKind.file);
  }
}

sealed class _ClipboardInsertion {
  const _ClipboardInsertion({required this.uploadId});

  final String uploadId;
  _ClipboardInsertionKind get kind;

  void insert({required UploadManager uploadManager, required void Function(Map<String, dynamic>) dispatch});
}

final class _ClipboardImageInsertion extends _ClipboardInsertion {
  const _ClipboardImageInsertion({
    required super.uploadId,
    required this.name,
    required this.bytes,
    required this.width,
    required this.height,
  });

  final String name;
  final Uint8List bytes;
  final int width;
  final int height;

  @override
  _ClipboardInsertionKind get kind => _ClipboardInsertionKind.image;

  @override
  void insert({required UploadManager uploadManager, required void Function(Map<String, dynamic>) dispatch}) {
    uploadManager.addInflightImage(uploadId, InflightImage(name: name, bytes: bytes, width: width, height: height));
    dispatch({'type': 'insertImage', 'uploadId': uploadId});
  }
}

final class _ClipboardFileInsertion extends _ClipboardInsertion {
  const _ClipboardFileInsertion({
    required super.uploadId,
    required this.name,
    required this.size,
    this.path,
    this.bytes,
    this.mimeType,
  }) : assert(path != null || bytes != null);

  final String? path;
  final Uint8List? bytes;
  final String? mimeType;
  final String name;
  final int size;

  @override
  _ClipboardInsertionKind get kind => _ClipboardInsertionKind.file;

  @override
  void insert({required UploadManager uploadManager, required void Function(Map<String, dynamic>) dispatch}) {
    uploadManager.addInflightFile(
      uploadId,
      InflightFile(path: path, bytes: bytes, mimeType: mimeType, name: name, size: size),
    );
    dispatch({'type': 'insertFile', 'uploadId': uploadId});
  }
}

enum _ClipboardInsertionKind { image, file }
