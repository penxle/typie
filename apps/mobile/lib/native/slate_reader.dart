import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'package:typie/native/editor_native.dart';

Map<String, int>? _cachedOffsets;

Map<String, int> _getOffsets() {
  return _cachedOffsets ??= getSlateOffsets();
}

const _tagItalic = 5;
const _tagStrikethrough = 9;
const _tagUnderline = 10;
const _tagFontSize = 2;
const _tagLetterSpacing = 6;
const _tagLineHeight = 21;
const _tagFontWeight = 4;
const _tagTextAlign = 20;
const _tagBackgroundColor = 0;
const _tagTextColor = 1;
const _tagFontFamily = 3;
const _tagLink = 30;
const _tagRuby = 31;

const _vkUnit = 0;
const _vkF32 = 1;
const _vkU32 = 2;
const _vkString = 3;
const _vkComposite = 4;

const _alignLeft = 0;
const _alignCenter = 1;
const _alignRight = 2;
const _alignJustify = 3;
const selectionTypeNone = 0;
const selectionTypeHorizontalRule = 1;
const selectionTypeCallout = 2;
const selectionTypeFold = 3;
const selectionTypeBulletList = 4;
const selectionTypeOrderedList = 5;
const selectionTypeImage = 6;
const selectionTypeFile = 7;
const selectionTypeEmbed = 8;
const selectionTypeArchived = 9;
const selectionTypeBlockquote = 10;

const _unitTagMap = {_tagItalic: 'italic', _tagStrikethrough: 'strikethrough', _tagUnderline: 'underline'};
const _f32TagMap = {_tagFontSize: 'font_size', _tagLetterSpacing: 'letter_spacing', _tagLineHeight: 'line_height'};
const _u32TagMap = {_tagFontWeight: 'font_weight', _tagTextAlign: 'text_align'};
const _stringTagMap = {
  _tagBackgroundColor: 'background_color',
  _tagTextColor: 'text_color',
  _tagFontFamily: 'font_family',
};
const _textAlignMap = {_alignLeft: 'left', _alignCenter: 'center', _alignRight: 'right', _alignJustify: 'justify'};

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
  int getU32OrZero(String field) {
    final offset = _offsets[field];
    if (offset == null) {
      return 0;
    }
    return _slateBytes.getUint32(offset, Endian.little);
  }

  List<String> get selectionBlockIds =>
      readNodeIdList(getU32OrZero('selection_block_ids_offset'), getU32OrZero('selection_block_ids_count'));
  List<int> get selectionBlockTypes =>
      readU32List(getU32OrZero('selection_block_types_offset'), getU32OrZero('selection_block_types_count'));
  List<String> get selectionCommonAncestorIds => readNodeIdList(
    getU32OrZero('selection_common_ancestor_ids_offset'),
    getU32OrZero('selection_common_ancestor_ids_count'),
  );
  List<int> get selectionCommonAncestorTypes => readU32List(
    getU32OrZero('selection_common_ancestor_types_offset'),
    getU32OrZero('selection_common_ancestor_types_count'),
  );

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

  List<String> readNodeIdList(int offset, int count) {
    if (count == 0) {
      return const [];
    }

    final result = List<String>.filled(count, '');
    for (var i = 0; i < count; i++) {
      final start = offset + i * 16;
      final end = start + 16;
      result[i] = _bytesToHex(_slabData.sublist(start, end));
    }
    return result;
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

  List<Map<String, dynamic>> readAttrs() {
    final count = getU32('attrs_count');
    if (count == 0) {
      return const [];
    }

    var pos = getU32('attrs_offset');
    final result = <Map<String, dynamic>>[];

    for (var i = 0; i < count; i++) {
      final typeTag = _slabU32(pos);
      final valueKind = _slabU32(pos + 4);
      final valueCount = _slabU32(pos + 8);
      pos += 12;

      if (valueKind == _vkUnit) {
        final values = <bool?>[];
        for (var j = 0; j < valueCount; j++) {
          final v = _slabU32(pos);
          pos += 4;
          values.add(v == 0xFFFFFFFF ? null : true);
        }
        final type = _unitTagMap[typeTag];
        if (type != null) result.add({'type': type, 'values': values});
      } else if (valueKind == _vkF32) {
        final values = <double?>[];
        for (var j = 0; j < valueCount; j++) {
          final v = _slabF32(pos);
          pos += 4;
          values.add(v.isNaN ? null : v);
        }
        final type = _f32TagMap[typeTag];
        if (type != null) result.add({'type': type, 'values': values});
      } else if (valueKind == _vkU32) {
        final values = <dynamic>[];
        for (var j = 0; j < valueCount; j++) {
          final v = _slabU32(pos);
          pos += 4;
          values.add(v == 0xFFFFFFFF ? null : v);
        }
        final type = _u32TagMap[typeTag];
        if (type != null) {
          if (type == 'text_align') {
            result.add({'type': type, 'values': values.map((v) => v == null ? null : _textAlignMap[v]).toList()});
          } else {
            result.add({'type': type, 'values': values});
          }
        }
      } else if (valueKind == _vkString) {
        final values = <String?>[];
        for (var j = 0; j < valueCount; j++) {
          final byteLen = _slabU32(pos);
          if (byteLen == 0xFFFFFFFF) {
            values.add(null);
            pos += 4;
          } else {
            values.add(readStr(pos));
            pos += _strByteLen(pos);
          }
        }
        final type = _stringTagMap[typeTag];
        if (type != null) result.add({'type': type, 'values': values});
      } else if (valueKind == _vkComposite) {
        if (typeTag == _tagLink) {
          final values = <Map<String, String>?>[];
          for (var j = 0; j < valueCount; j++) {
            final fieldCount = _slabU32(pos);
            pos += 4;
            if (fieldCount == 0xFFFFFFFF) {
              values.add(null);
            } else {
              final obj = <String, String>{};
              for (var k = 0; k < fieldCount; k++) {
                final fvk = _slabU32(pos);
                pos += 4;
                if (fvk == _vkString) {
                  final value = readStr(pos);
                  pos += _strByteLen(pos);
                  obj[k == 0 ? 'href' : 'field_$k'] = value;
                }
              }
              values.add(obj);
            }
          }
          result.add({'type': 'link', 'values': values});
        } else if (typeTag == _tagRuby) {
          final values = <Map<String, String>?>[];
          for (var j = 0; j < valueCount; j++) {
            final fieldCount = _slabU32(pos);
            pos += 4;
            if (fieldCount == 0xFFFFFFFF) {
              values.add(null);
            } else {
              final obj = <String, String>{};
              for (var k = 0; k < fieldCount; k++) {
                final fvk = _slabU32(pos);
                pos += 4;
                if (fvk == _vkString) {
                  final value = readStr(pos);
                  pos += _strByteLen(pos);
                  obj[k == 0 ? 'text' : 'field_$k'] = value;
                }
              }
              values.add(obj);
            }
          }
          result.add({'type': 'ruby', 'values': values});
        }
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
