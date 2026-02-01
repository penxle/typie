import 'dart:typed_data';

import 'package:hive_ce/hive.dart';
import 'package:path_provider/path_provider.dart';

const _boxName = 'document_sync';

class _StoredDocument {
  _StoredDocument({required this.id, required this.snapshot, required this.pendingUpdates, required this.updatedAt});

  final String id;
  final Uint8List? snapshot;
  final List<Uint8List> pendingUpdates;
  final int updatedAt;

  Map<String, dynamic> toMap() {
    return {'id': id, 'snapshot': snapshot, 'pendingUpdates': pendingUpdates, 'updatedAt': updatedAt};
  }

  static _StoredDocument? fromMap(Map<dynamic, dynamic>? map) {
    if (map == null) {
      return null;
    }
    return _StoredDocument(
      id: map['id'] as String,
      snapshot: map['snapshot'] as Uint8List?,
      pendingUpdates: (map['pendingUpdates'] as List?)?.cast<Uint8List>() ?? [],
      updatedAt: map['updatedAt'] as int,
    );
  }
}

class DocumentLocalPersistence {
  DocumentLocalPersistence(this.documentId);

  final String documentId;
  Box<dynamic>? _box;
  bool _disposed = false;

  Future<void> _ensureBox() async {
    if (_box != null) {
      return;
    }

    final dir = await getApplicationDocumentsDirectory();
    final path = '${dir.path}/hive';
    Hive.init(path);

    _box = await Hive.openBox(_boxName);
  }

  Future<({Uint8List? snapshot, List<Uint8List> pendingUpdates})?> load() async {
    if (_disposed) {
      return null;
    }

    await _ensureBox();
    final data = _box!.get(documentId);
    final doc = _StoredDocument.fromMap(data as Map<dynamic, dynamic>?);

    if (doc == null) {
      return null;
    }
    return (snapshot: doc.snapshot, pendingUpdates: doc.pendingUpdates);
  }

  Future<void> storeUpdate(Uint8List update) async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    final existing = _StoredDocument.fromMap(_box!.get(documentId) as Map<dynamic, dynamic>?);

    final doc = _StoredDocument(
      id: documentId,
      snapshot: existing?.snapshot,
      pendingUpdates: [...(existing?.pendingUpdates ?? []), update],
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );

    await _box!.put(documentId, doc.toMap());
  }

  Future<void> saveSnapshot(Uint8List snapshot) async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    final doc = _StoredDocument(
      id: documentId,
      snapshot: snapshot,
      pendingUpdates: [],
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );

    await _box!.put(documentId, doc.toMap());
  }

  Future<void> clear() async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    await _box!.delete(documentId);
  }

  void dispose() {
    _disposed = true;
  }
}
