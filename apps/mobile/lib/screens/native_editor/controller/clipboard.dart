import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';

class EditorClipboard {
  static const _channel = MethodChannel('co.typie.clipboard');

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
}
