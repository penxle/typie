import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'package:typie/native/editor_native.dart';

Map<String, int>? _cachedOffsets;

Map<String, int> _getOffsets() {
  return _cachedOffsets ??= getSlateOffsets();
}

const _styleBits = [
  ('background_color', 1 << 0),
  ('text_color', 1 << 1),
  ('font_size', 1 << 2),
  ('font_family', 1 << 3),
  ('font_weight', 1 << 4),
  ('italic', 1 << 5),
  ('letter_spacing', 1 << 6),
  ('strikethrough', 1 << 9),
  ('underline', 1 << 10),
];

class SlateReader {
  SlateReader(Pointer<Uint8> slatePtr, int slateLen, Pointer<Uint8> slabPtr, int slabLen)
    : _offsets = _getOffsets(),
      _slateData = slatePtr.asTypedList(slateLen),
      _slateBytes = slatePtr.asTypedList(slateLen).buffer.asByteData(),
      _slabData = slabLen > 0 ? slabPtr.asTypedList(slabLen) : Uint8List(0),
      _slabBytes = slabLen > 0 ? slabPtr.asTypedList(slabLen).buffer.asByteData() : ByteData(0);

  final Map<String, int> _offsets;
  final Uint8List _slateData;
  final ByteData _slateBytes;
  final Uint8List _slabData;
  final ByteData _slabBytes;

  int _offset(String field) => _offsets[field]!;

  int get dirty {
    final off = _offset('dirty');
    return _slateBytes.getUint32(off, Endian.little) | (_slateBytes.getUint32(off + 4, Endian.little) << 32);
  }

  bool isDirty(int bit) => (dirty & (1 << bit)) != 0;

  int getU32(String field) => _slateBytes.getUint32(_offset(field), Endian.little);

  double getF32(String field) => _slateBytes.getFloat32(_offset(field), Endian.little);

  int getI32(String field) => _slateBytes.getInt32(_offset(field), Endian.little);

  String readStr(int offset) {
    final byteLen = _slabBytes.getUint32(offset, Endian.little);
    return utf8.decode(_slabData.sublist(offset + 4, offset + 4 + byteLen));
  }

  List<double> readF32List(int offset, int count) {
    if (count == 0) {
      return const [];
    }
    final result = List<double>.filled(count, 0);
    for (var i = 0; i < count; i++) {
      result[i] = _slabBytes.getFloat32(offset + i * 4, Endian.little);
    }
    return result;
  }

  List<int> readU32List(int offset, int count) {
    if (count == 0) {
      return const [];
    }
    final result = List<int>.filled(count, 0);
    for (var i = 0; i < count; i++) {
      result[i] = _slabBytes.getUint32(offset + i * 4, Endian.little);
    }
    return result;
  }

  String readNodeId(int slateOffset) {
    final bytes = _slateData.sublist(slateOffset, slateOffset + 16);
    return _bytesToHex(bytes);
  }

  String readNodeIdField(String field) {
    return readNodeId(_offset(field));
  }

  int _slabU32(int offset) => _slabBytes.getUint32(offset, Endian.little);
  int slabU32(int offset) => _slabU32(offset);
  double _slabF32(int offset) => _slabBytes.getFloat32(offset, Endian.little);
  double slabF32(int offset) => _slabF32(offset);

  int _strByteLen(int offset) {
    final byteLen = _slabU32(offset);
    final total = 4 + byteLen;
    final aligned = (total + 3) & ~3;
    return aligned;
  }

  List<Map<String, dynamic>> readUniformStyles() {
    final count = getU32('formatting_uniform_styles_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('formatting_uniform_styles_offset');
    final result = <Map<String, dynamic>>[];

    for (var i = 0; i < count; i++) {
      final typeTag = _slabU32(pos);
      final valueKind = _slabU32(pos + 4);
      pos += 8;

      var strValue = '';
      num numValue = 0;

      switch (valueKind) {
        case 0:
          break;
        case 1:
          numValue = _slabF32(pos);
          pos += 4;
        case 2:
          numValue = _slabU32(pos);
          pos += 4;
        case 3:
          strValue = readStr(pos);
          pos += _strByteLen(pos);
      }

      final Map<String, dynamic>? style;
      switch (typeTag) {
        case 0:
          style = {'type': 'background_color', 'color': strValue};
        case 1:
          style = {'type': 'text_color', 'color': strValue};
        case 2:
          style = {'type': 'font_size', 'size': numValue};
        case 3:
          style = {'type': 'font_family', 'family': strValue};
        case 4:
          style = {'type': 'font_weight', 'weight': numValue};
        case 5:
          style = {'type': 'italic'};
        case 6:
          style = {'type': 'letter_spacing', 'spacing': numValue};
        case 9:
          style = {'type': 'strikethrough'};
        case 10:
          style = {'type': 'underline'};
        default:
          style = null;
      }

      if (style != null) {
        result.add(style);
      }
    }

    return result;
  }

  List<String> readMixedStyles() {
    final bitfield = getU32('formatting_mixed_styles_bitfield');
    if (bitfield == 0) {
      return const [];
    }

    final result = <String>[];
    for (final (name, bit) in _styleBits) {
      if (bitfield & bit != 0) {
        result.add(name);
      }
    }
    return result;
  }

  List<({String family, int weight, List<int> codepoints})> readFontRequests() {
    final count = getU32('font_requests_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('font_requests_offset');
    final result = <({String family, int weight, List<int> codepoints})>[];

    for (var i = 0; i < count; i++) {
      final family = readStr(pos);
      pos += _strByteLen(pos);

      final weight = _slabU32(pos);
      pos += 4;

      final (codepoints, newPos) = _readU32SliceRaw(pos);
      pos = newPos;

      result.add((family: family, weight: weight, codepoints: codepoints));
    }

    return result;
  }

  (List<int>, int) _readU32SliceRaw(int offset) {
    final count = _slabU32(offset);
    final values = readU32List(offset + 4, count);
    return (values, offset + 4 + count * 4);
  }

  List<int> readFallbackCodepoints() {
    final count = getU32('fallback_codepoints_count');
    if (count == 0) {
      return const [];
    }
    return readU32List(getU32('fallback_codepoints_offset'), count);
  }

  List<_ExternalElementRaw> readExternalElements() {
    final count = getU32('external_elements_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('external_elements_offset');
    final result = <_ExternalElementRaw>[];

    for (var i = 0; i < count; i++) {
      final pageIdx = _slabU32(pos);
      pos += 4;

      final nodeId = readStr(pos);
      pos += _strByteLen(pos);

      final x = _slabF32(pos);
      final y = _slabF32(pos + 4);
      final w = _slabF32(pos + 8);
      final h = _slabF32(pos + 12);
      pos += 16;

      final isSelected = _slabU32(pos) != 0;
      pos += 4;

      final dataTag = _slabU32(pos);
      pos += 4;

      String? id;
      String? uploadId;
      var proportion = 0.0;

      switch (dataTag) {
        case 0:
          final rawId = readStr(pos);
          pos += _strByteLen(pos);
          final rawUploadId = readStr(pos);
          pos += _strByteLen(pos);
          proportion = _slabF32(pos);
          pos += 4;
          id = rawId.isEmpty ? null : rawId;
          uploadId = rawUploadId.isEmpty ? null : rawUploadId;
        case 1:
          final rawId = readStr(pos);
          pos += _strByteLen(pos);
          final rawUploadId = readStr(pos);
          pos += _strByteLen(pos);
          id = rawId.isEmpty ? null : rawId;
          uploadId = rawUploadId.isEmpty ? null : rawUploadId;
        default:
          final rawId = readStr(pos);
          pos += _strByteLen(pos);
          id = rawId.isEmpty ? null : rawId;
      }

      result.add(
        _ExternalElementRaw(
          pageIdx: pageIdx,
          nodeId: nodeId,
          x: x,
          y: y,
          width: w,
          height: h,
          isSelected: isSelected,
          dataTag: dataTag,
          id: id,
          uploadId: uploadId,
          proportion: proportion,
        ),
      );
    }

    return result;
  }

  List<_TrackedItemOverlayRaw> readTrackedItems() {
    final count = getU32('tracked_items_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('tracked_items_offset');
    final result = <_TrackedItemOverlayRaw>[];

    for (var i = 0; i < count; i++) {
      final pageIdx = _slabU32(pos);
      pos += 4;

      final group = _slabU32(pos);
      pos += 4;

      final id = readStr(pos);
      pos += _strByteLen(pos);

      final nodeIdByteLen = _slabU32(pos);
      pos += 4;
      final nodeIdBytes = _slabData.sublist(pos, pos + nodeIdByteLen);
      final nodeId = _bytesToHex(nodeIdBytes);
      final alignPad = (4 - ((pos + nodeIdByteLen) % 4)) % 4;
      pos += nodeIdByteLen + alignPad;

      final startOffset = _slabU32(pos);
      final endOffset = _slabU32(pos + 4);
      pos += 8;

      final boundsCount = _slabU32(pos);
      pos += 4;

      final bounds = <_TextBoundRaw>[];
      for (var j = 0; j < boundsCount; j++) {
        bounds.add(
          _TextBoundRaw(
            x: _slabF32(pos),
            y: _slabF32(pos + 4),
            width: _slabF32(pos + 8),
            height: _slabF32(pos + 12),
            ascent: _slabF32(pos + 16),
          ),
        );
        pos += 20;
      }

      result.add(
        _TrackedItemOverlayRaw(
          pageIdx: pageIdx,
          group: group,
          id: id,
          nodeId: nodeId,
          startOffset: startOffset,
          endOffset: endOffset,
          bounds: bounds,
        ),
      );
    }

    return result;
  }

  _HtmlPastedRaw? readHtmlPasted() {
    final textLen = getU32('html_pasted_len');
    if (textLen == 0) {
      return null;
    }

    var pos = getU32('html_pasted_offset');

    final text = readStr(pos);
    pos += _strByteLen(pos);

    final fromNodeByteLen = _slabU32(pos);
    pos += 4;
    final fromNodeBytes = _slabData.sublist(pos, pos + fromNodeByteLen);
    pos += fromNodeByteLen;
    final alignPad1 = (4 - (pos % 4)) % 4;
    pos += alignPad1;

    final fromOffset = _slabU32(pos);
    final fromAffinity = _slabU32(pos + 4);
    pos += 8;

    final toNodeByteLen = _slabU32(pos);
    pos += 4;
    final toNodeBytes = _slabData.sublist(pos, pos + toNodeByteLen);
    pos += toNodeByteLen;
    final alignPad2 = (4 - (pos % 4)) % 4;
    pos += alignPad2;

    final toOffset = _slabU32(pos);
    final toAffinity = _slabU32(pos + 4);

    return _HtmlPastedRaw(
      text: text,
      fromNodeId: _bytesToHex(fromNodeBytes),
      fromOffset: fromOffset,
      fromAffinity: fromAffinity == 1 ? 'downstream' : 'upstream',
      toNodeId: _bytesToHex(toNodeBytes),
      toOffset: toOffset,
      toAffinity: toAffinity == 1 ? 'downstream' : 'upstream',
    );
  }

  List<_LinkOverlayRaw> readLinkOverlays() {
    final count = getU32('link_overlays_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('link_overlays_offset');
    final result = <_LinkOverlayRaw>[];

    for (var i = 0; i < count; i++) {
      final pageIdx = _slabU32(pos);
      pos += 4;

      final href = readStr(pos);
      pos += _strByteLen(pos);

      final boundsCount = _slabU32(pos);
      pos += 4;

      final bounds = <_TextBoundRaw>[];
      for (var j = 0; j < boundsCount; j++) {
        bounds.add(
          _TextBoundRaw(
            x: _slabF32(pos),
            y: _slabF32(pos + 4),
            width: _slabF32(pos + 8),
            height: _slabF32(pos + 12),
            ascent: _slabF32(pos + 16),
          ),
        );
        pos += 20;
      }

      result.add(_LinkOverlayRaw(pageIdx: pageIdx, href: href, bounds: bounds));
    }

    return result;
  }

  List<String> readEnabledActions() {
    final count = getU32('enabled_actions_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('enabled_actions_offset');
    final result = <String>[];

    for (var i = 0; i < count; i++) {
      result.add(readStr(pos));
      pos += _strByteLen(pos);
    }

    return result;
  }

  static String _bytesToHex(Uint8List bytes) {
    final hex = StringBuffer();
    for (final b in bytes) {
      hex.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return hex.toString();
  }
}

class _ExternalElementRaw {
  const _ExternalElementRaw({
    required this.pageIdx,
    required this.nodeId,
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    required this.isSelected,
    required this.dataTag,
    required this.id,
    required this.uploadId,
    required this.proportion,
  });

  final int pageIdx;
  final String nodeId;
  final double x;
  final double y;
  final double width;
  final double height;
  final bool isSelected;
  final int dataTag;
  final String? id;
  final String? uploadId;
  final double proportion;
}

class _TextBoundRaw {
  const _TextBoundRaw({
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    required this.ascent,
  });

  final double x;
  final double y;
  final double width;
  final double height;
  final double ascent;
}

class _TrackedItemOverlayRaw {
  const _TrackedItemOverlayRaw({
    required this.pageIdx,
    required this.group,
    required this.id,
    required this.nodeId,
    required this.startOffset,
    required this.endOffset,
    required this.bounds,
  });

  final int pageIdx;
  final int group;
  final String id;
  final String nodeId;
  final int startOffset;
  final int endOffset;
  final List<_TextBoundRaw> bounds;
}

class _HtmlPastedRaw {
  const _HtmlPastedRaw({
    required this.text,
    required this.fromNodeId,
    required this.fromOffset,
    required this.fromAffinity,
    required this.toNodeId,
    required this.toOffset,
    required this.toAffinity,
  });

  final String text;
  final String fromNodeId;
  final int fromOffset;
  final String fromAffinity;
  final String toNodeId;
  final int toOffset;
  final String toAffinity;
}

class _LinkOverlayRaw {
  const _LinkOverlayRaw({required this.pageIdx, required this.href, required this.bounds});

  final int pageIdx;
  final String href;
  final List<_TextBoundRaw> bounds;
}
