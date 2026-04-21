import 'package:flutter/foundation.dart';
import 'package:typie/screens/native_editor/external/models.dart';

class UploadManager extends ChangeNotifier {
  final Map<String, InflightImage> _inflightImages = {};
  final Map<String, InflightFile> _inflightFiles = {};
  final Map<String, ImageAsset> _imageAssets = {};
  final Map<String, FileAsset> _fileAssets = {};
  final Map<String, EmbedAsset> _embedAssets = {};
  final Map<String, ArchivedAsset> _archivedAssets = {};
  final Map<String, String> _localImageUploadIds = {};
  final Map<String, String> _localFileUploadIds = {};
  final Map<String, bool> _inflightEmbeds = {};
  bool _disposed = false;

  bool get isDisposed => _disposed;

  Map<String, InflightImage> get inflightImages => Map.unmodifiable(_inflightImages);
  Map<String, InflightFile> get inflightFiles => Map.unmodifiable(_inflightFiles);
  Map<String, ImageAsset> get imageAssets => Map.unmodifiable(_imageAssets);
  Map<String, FileAsset> get fileAssets => Map.unmodifiable(_fileAssets);
  Map<String, EmbedAsset> get embedAssets => Map.unmodifiable(_embedAssets);
  Map<String, ArchivedAsset> get archivedAssets => Map.unmodifiable(_archivedAssets);
  Map<String, String> get localImageUploadIds => Map.unmodifiable(_localImageUploadIds);
  Map<String, String> get localFileUploadIds => Map.unmodifiable(_localFileUploadIds);
  Map<String, bool> get inflightEmbeds => Map.unmodifiable(_inflightEmbeds);

  void addInflightImage(String uploadId, InflightImage image) {
    if (_disposed) {
      return;
    }
    _inflightImages[uploadId] = image;
    _notifyListeners();
  }

  void removeInflightImage(String uploadId) {
    if (_disposed) {
      return;
    }
    _inflightImages.remove(uploadId);
    _notifyListeners();
  }

  void addImageAsset(String id, ImageAsset asset) {
    if (_disposed) {
      return;
    }
    _imageAssets[id] = asset;
    _notifyListeners();
  }

  void setLocalImageUploadId(String nodeId, String uploadId) {
    if (_disposed) {
      return;
    }
    _localImageUploadIds[nodeId] = uploadId;
    _notifyListeners();
  }

  void removeLocalImageUploadId(String nodeId) {
    if (_disposed) {
      return;
    }
    _localImageUploadIds.remove(nodeId);
    _notifyListeners();
  }

  void completeImageUpload({required String uploadId, required String nodeId, required ImageAsset asset}) {
    if (_disposed) {
      return;
    }
    _inflightImages.remove(uploadId);
    _localImageUploadIds.remove(nodeId);
    _imageAssets[asset.id] = asset;
    _notifyListeners();
  }

  void failImageUpload({required String uploadId, required String nodeId}) {
    if (_disposed) {
      return;
    }
    _inflightImages.remove(uploadId);
    _localImageUploadIds.remove(nodeId);
    _notifyListeners();
  }

  void addInflightFile(String uploadId, InflightFile file) {
    if (_disposed) {
      return;
    }
    _inflightFiles[uploadId] = file;
    _notifyListeners();
  }

  void removeInflightFile(String uploadId) {
    if (_disposed) {
      return;
    }
    _inflightFiles.remove(uploadId);
    _notifyListeners();
  }

  void addFileAsset(String id, FileAsset asset) {
    if (_disposed) {
      return;
    }
    _fileAssets[id] = asset;
    _notifyListeners();
  }

  void setLocalFileUploadId(String nodeId, String uploadId) {
    if (_disposed) {
      return;
    }
    _localFileUploadIds[nodeId] = uploadId;
    _notifyListeners();
  }

  void removeLocalFileUploadId(String nodeId) {
    if (_disposed) {
      return;
    }
    _localFileUploadIds.remove(nodeId);
    _notifyListeners();
  }

  void completeFileUpload({required String uploadId, required String nodeId, required FileAsset asset}) {
    if (_disposed) {
      return;
    }
    _inflightFiles.remove(uploadId);
    _localFileUploadIds.remove(nodeId);
    _fileAssets[asset.id] = asset;
    _notifyListeners();
  }

  void failFileUpload({required String uploadId, required String nodeId}) {
    if (_disposed) {
      return;
    }
    _inflightFiles.remove(uploadId);
    _localFileUploadIds.remove(nodeId);
    _notifyListeners();
  }

  void setInflightEmbed(String nodeId, {required bool inflight}) {
    if (_disposed) {
      return;
    }
    _inflightEmbeds[nodeId] = inflight;
    _notifyListeners();
  }

  void removeInflightEmbed(String nodeId) {
    if (_disposed) {
      return;
    }
    _inflightEmbeds.remove(nodeId);
    _notifyListeners();
  }

  void addEmbedAsset(String id, EmbedAsset asset) {
    if (_disposed) {
      return;
    }
    _embedAssets[id] = asset;
    _notifyListeners();
  }

  void addArchivedAsset(String id, ArchivedAsset asset) {
    if (_disposed) {
      return;
    }
    _archivedAssets[id] = asset;
    _notifyListeners();
  }

  void completeEmbedUnfurl({required String nodeId, required EmbedAsset asset}) {
    if (_disposed) {
      return;
    }
    _inflightEmbeds.remove(nodeId);
    _embedAssets[asset.id] = asset;
    _notifyListeners();
  }

  void failEmbedUnfurl({required String nodeId}) {
    if (_disposed) {
      return;
    }
    _inflightEmbeds.remove(nodeId);
    _notifyListeners();
  }

  void _notifyListeners() {
    if (!_disposed) {
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _inflightImages.clear();
    _inflightFiles.clear();
    _imageAssets.clear();
    _fileAssets.clear();
    _embedAssets.clear();
    _archivedAssets.clear();
    _localImageUploadIds.clear();
    _localFileUploadIds.clear();
    _inflightEmbeds.clear();
    super.dispose();
  }
}
