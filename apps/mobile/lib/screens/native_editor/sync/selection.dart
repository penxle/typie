import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/services/state.dart';

class SelectionSyncManager {
  SelectionSyncManager({required this.appState, required this.slug});

  final AppState appState;
  final String slug;

  Timer? _debounceTimer;
  VoidCallback? _titleFocusListener;
  VoidCallback? _subtitleFocusListener;

  void save(String data) {
    unawaited(
      appState.setSerializedDocumentSelection(slug, data).catchError((Object e) {
        if (e is FileSystemException && e.osError?.errorCode == 28) {
          return null;
        }
        return Future<void>.error(e);
      }),
    );
  }

  void saveElementFocus(String element) {
    _debounceTimer?.cancel();
    save(jsonEncode({'type': 'element', 'element': element}));
  }

  void handleSelectionChanged(
    Map<String, dynamic> anchor,
    Map<String, dynamic> head,
    bool Function() editorReady,
    bool Function() isFocused,
  ) {
    if (!editorReady() || !isFocused()) {
      return;
    }
    _debounceTimer?.cancel();
    _debounceTimer = Timer(const Duration(milliseconds: 150), () {
      if (!editorReady() || !isFocused()) {
        return;
      }
      save(
        jsonEncode({
          'selection': {
            'anchor': {'nodeId': anchor['nodeId'], 'offset': anchor['offset'], 'affinity': anchor['affinity']},
            'head': {'nodeId': head['nodeId'], 'offset': head['offset'], 'affinity': head['affinity']},
          },
        }),
      );
    });
  }

  void restore({
    required EditorController? controller,
    required FocusNode titleFocusNode,
    required FocusNode subtitleFocusNode,
  }) {
    final saved = appState.getSerializedDocumentSelection(slug);
    if (saved == null) {
      titleFocusNode.requestFocus();
      return;
    }

    try {
      final data = jsonDecode(saved) as Map<String, dynamic>;

      if (data['type'] == 'element') {
        final element = data['element'] as String;
        if (element == 'title') {
          titleFocusNode.requestFocus();
        } else if (element == 'subtitle') {
          subtitleFocusNode.requestFocus();
        }
        return;
      }

      final selection = data['selection'] as Map<String, dynamic>?;
      if (selection != null && controller != null) {
        final savedAnchor = selection['anchor'] as Map<String, dynamic>;
        final savedHead = selection['head'] as Map<String, dynamic>;
        controller
          ..dispatch({
            'type': 'setSelection',
            'anchorNodeId': savedAnchor['nodeId'],
            'anchorOffset': savedAnchor['offset'],
            'anchorAffinity': savedAnchor['affinity'],
            'headNodeId': savedHead['nodeId'],
            'headOffset': savedHead['offset'],
            'headAffinity': savedHead['affinity'],
          })
          ..requestFocus()
          ..scrollIntoView(mode: ScrollMode.typewriter, waitForCursorUpdate: true);
      }
    } catch (err) {
      titleFocusNode.requestFocus();
    }
  }

  void setupFocusListeners(FocusNode titleFocusNode, FocusNode subtitleFocusNode, bool Function() isEditorReady) {
    _titleFocusListener = () {
      if (titleFocusNode.hasFocus && isEditorReady()) {
        _debounceTimer?.cancel();
        saveElementFocus('title');
      }
    };

    _subtitleFocusListener = () {
      if (subtitleFocusNode.hasFocus && isEditorReady()) {
        _debounceTimer?.cancel();
        saveElementFocus('subtitle');
      }
    };

    titleFocusNode.addListener(_titleFocusListener!);
    subtitleFocusNode.addListener(_subtitleFocusListener!);
  }

  void dispose(FocusNode titleFocusNode, FocusNode subtitleFocusNode) {
    if (_titleFocusListener != null) {
      titleFocusNode.removeListener(_titleFocusListener!);
      _titleFocusListener = null;
    }
    if (_subtitleFocusListener != null) {
      subtitleFocusNode.removeListener(_subtitleFocusListener!);
      _subtitleFocusListener = null;
    }
    _debounceTimer?.cancel();
    _debounceTimer = null;
  }
}
