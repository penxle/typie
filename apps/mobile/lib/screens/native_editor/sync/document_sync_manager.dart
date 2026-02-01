import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/document_sync_stream.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/document_sync_stream.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/sync_document.req.gql.dart';
import 'package:typie/screens/native_editor/sync/local_persistence.dart';
import 'package:uuid/uuid.dart';

enum SyncStatus { syncing, synced, error }

class DocumentSyncManager {
  DocumentSyncManager({required this.documentId, required this.editor, required this.client});

  final String documentId;
  final NativeEditor editor;
  final GraphQLClient client;

  final String _clientId = const Uuid().v4();
  final ValueNotifier<SyncStatus> syncStatus = ValueNotifier(SyncStatus.synced);

  Timer? _syncUpdateTimer;
  Timer? _forceSyncTimer;
  Timer? _fullSyncTimer;
  StreamSubscription<dynamic>? _subscription;
  DocumentLocalPersistence? _persistence;

  bool _disposed = false;

  Future<void> start() async {
    _persistence = DocumentLocalPersistence(documentId);

    final local = await _persistence!.load();
    if (local != null) {
      final updates = [if (local.snapshot != null) local.snapshot!, ...local.pendingUpdates];
      if (updates.isNotEmpty) {
        editor.importUpdatesBatch(updates);
      }
    }

    await fullSync();

    _subscription = client
        .subscribe(
          GNativeEditor_DocumentSyncStream_SubscriptionReq(
            (b) => b
              ..vars.clientId = _clientId
              ..vars.documentId = documentId,
          ),
        )
        .listen(_handleSyncMessage);

    _forceSyncTimer = Timer.periodic(const Duration(seconds: 10), (_) => forceSync());
    _fullSyncTimer = Timer.periodic(const Duration(seconds: 60), (_) => fullSync());
  }

  void handleDocChanged() {
    if (_disposed) {
      return;
    }

    syncStatus.value = SyncStatus.syncing;

    final result = editor.exportNewUpdates();
    if (result != null && _persistence != null) {
      unawaited(_persistence!.storeUpdate(result.updates));
    }

    _syncUpdateTimer?.cancel();
    _syncUpdateTimer = Timer(const Duration(seconds: 1), _sendUpdates);
  }

  Future<void> _sendUpdates() async {
    if (_disposed) {
      return;
    }

    final result = editor.exportNewUpdates();
    if (result == null || result.updates.isEmpty) {
      if (!_disposed) {
        syncStatus.value = SyncStatus.synced;
      }
      return;
    }

    try {
      await client.request(
        GNativeEditor_SyncDocument_MutationReq(
          (b) => b
            ..vars.input.clientId = _clientId
            ..vars.input.documentId = documentId
            ..vars.input.type = GDocumentSyncType.UPDATE
            ..vars.input.data = base64Encode(result.updates),
        ),
      );
      if (_disposed) {
        return;
      }
      editor.commitSync(result.version);
      syncStatus.value = SyncStatus.synced;
    } catch (err) {
      debugPrint('Sync error: $err');
      if (!_disposed) {
        syncStatus.value = SyncStatus.error;
      }
    }
  }

  Future<void> fullSync() async {
    if (_disposed) {
      return;
    }

    final update = editor.exportAllUpdates();
    if (update == null) {
      return;
    }

    final snapshot = editor.getSnapshot();
    final version = editor.getVersion();

    if (_persistence != null && snapshot != null) {
      await _persistence!.saveSnapshot(snapshot);
    }

    if (_disposed) {
      return;
    }

    try {
      await client.request(
        GNativeEditor_SyncDocument_MutationReq(
          (b) => b
            ..vars.input.clientId = _clientId
            ..vars.input.documentId = documentId
            ..vars.input.type = GDocumentSyncType.UPDATE
            ..vars.input.data = base64Encode(update),
        ),
      );

      if (_disposed) {
        return;
      }

      if (version != null) {
        editor.commitSync(version);
      }
    } catch (err) {
      debugPrint('Full sync error: $err');
    }
  }

  Future<void> forceSync() async {
    if (_disposed) {
      return;
    }

    final version = editor.getVersion();
    if (version == null) {
      return;
    }

    try {
      await client.request(
        GNativeEditor_SyncDocument_MutationReq(
          (b) => b
            ..vars.input.clientId = _clientId
            ..vars.input.documentId = documentId
            ..vars.input.type = GDocumentSyncType.VECTOR
            ..vars.input.data = base64Encode(version),
        ),
      );
    } catch (err) {
      debugPrint('Force sync error: $err');
    }
  }

  void _handleSyncMessage(GNativeEditor_DocumentSyncStream_SubscriptionData data) {
    if (_disposed) {
      return;
    }

    final payload = data.documentSyncStream;

    if (payload.type == GDocumentSyncType.HEARTBEAT) {
      syncStatus.value = SyncStatus.synced;
    } else if (payload.type == GDocumentSyncType.UPDATE) {
      final updates = base64Decode(payload.data);
      editor.importUpdates(Uint8List.fromList(updates));
    } else if (payload.type == GDocumentSyncType.VECTOR) {
      final versionBytes = base64Decode(payload.data);
      editor.commitSync(Uint8List.fromList(versionBytes));
    }
  }

  void dispose() {
    _disposed = true;
    _syncUpdateTimer?.cancel();
    _forceSyncTimer?.cancel();
    _fullSyncTimer?.cancel();
    unawaited(_subscription?.cancel());

    final result = editor.exportNewUpdates();
    if (result != null && result.updates.isNotEmpty) {
      unawaited(
        client.request(
          GNativeEditor_SyncDocument_MutationReq(
            (b) => b
              ..vars.input.clientId = _clientId
              ..vars.input.documentId = documentId
              ..vars.input.type = GDocumentSyncType.UPDATE
              ..vars.input.data = base64Encode(result.updates),
          ),
        ),
      );
    }

    _persistence?.dispose();
    syncStatus.dispose();
  }
}
