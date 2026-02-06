import 'dart:async';

import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_mutation.req.gql.dart';

class TitleSyncManager {
  TitleSyncManager({required this.documentId, required this.client});

  final String documentId;
  final GraphQLClient client;

  String title = '';
  String subtitle = '';
  bool titleDirty = false;
  bool subtitleDirty = false;
  Timer? _titleDebounceTimer;
  Timer? _subtitleDebounceTimer;

  void updateFromServer(String? serverTitle, String? serverSubtitle) {
    final newTitle = serverTitle ?? '';
    final newSubtitle = serverSubtitle ?? '';

    if (titleDirty && newTitle == title) {
      titleDirty = false;
    }
    if (subtitleDirty && newSubtitle == subtitle) {
      subtitleDirty = false;
    }

    if (!titleDirty) {
      title = newTitle;
    }
    if (!subtitleDirty) {
      subtitle = newSubtitle;
    }
  }

  void handleTitleChanged(String value) {
    title = value;
    titleDirty = true;
    _titleDebounceTimer?.cancel();
    _titleDebounceTimer = Timer(const Duration(milliseconds: 200), () {
      _saveTitle(value);
    });
  }

  void handleSubtitleChanged(String value) {
    subtitle = value;
    subtitleDirty = true;
    _subtitleDebounceTimer?.cancel();
    _subtitleDebounceTimer = Timer(const Duration(milliseconds: 200), () {
      _saveSubtitle(value);
    });
  }

  void flush() {
    if (_titleDebounceTimer != null) {
      _titleDebounceTimer!.cancel();
      if (titleDirty) {
        _saveTitle(title);
      }
    }
    if (_subtitleDebounceTimer != null) {
      _subtitleDebounceTimer!.cancel();
      if (subtitleDirty) {
        _saveSubtitle(subtitle);
      }
    }
  }

  void dispose() {
    flush();
    _titleDebounceTimer = null;
    _subtitleDebounceTimer = null;
  }

  void _saveTitle(String value) {
    unawaited(
      client.request(
        GNativeEditor_UpdateDocument_MutationReq(
          (b) => b.vars.input
            ..documentId = documentId
            ..title = Value.present(value.isEmpty ? null : value),
        ),
      ),
    );
  }

  void _saveSubtitle(String value) {
    unawaited(
      client.request(
        GNativeEditor_UpdateDocument_MutationReq(
          (b) => b.vars.input
            ..documentId = documentId
            ..subtitle = Value.present(value.isEmpty ? null : value),
        ),
      ),
    );
  }
}
