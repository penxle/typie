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
    _inflightImages[uploadId] = image;
    notifyListeners();
  }

  void removeInflightImage(String uploadId) {
    _inflightImages.remove(uploadId);
    notifyListeners();
  }

  void addImageAsset(String id, ImageAsset asset) {
    _imageAssets[id] = asset;
    notifyListeners();
  }

  void setLocalImageUploadId(String nodeId, String uploadId) {
    _localImageUploadIds[nodeId] = uploadId;
    notifyListeners();
  }

  void removeLocalImageUploadId(String nodeId) {
    _localImageUploadIds.remove(nodeId);
    notifyListeners();
  }

  void completeImageUpload({required String uploadId, required String nodeId, required ImageAsset asset}) {
    _inflightImages.remove(uploadId);
    _localImageUploadIds.remove(nodeId);
    _imageAssets[asset.id] = asset;
    notifyListeners();
  }

  void failImageUpload({required String uploadId, required String nodeId}) {
    _inflightImages.remove(uploadId);
    _localImageUploadIds.remove(nodeId);
    notifyListeners();
  }

  void addInflightFile(String uploadId, InflightFile file) {
    _inflightFiles[uploadId] = file;
    notifyListeners();
  }

  void removeInflightFile(String uploadId) {
    _inflightFiles.remove(uploadId);
    notifyListeners();
  }

  void addFileAsset(String id, FileAsset asset) {
    _fileAssets[id] = asset;
    notifyListeners();
  }

  void setLocalFileUploadId(String nodeId, String uploadId) {
    _localFileUploadIds[nodeId] = uploadId;
    notifyListeners();
  }

  void removeLocalFileUploadId(String nodeId) {
    _localFileUploadIds.remove(nodeId);
    notifyListeners();
  }

  void completeFileUpload({required String uploadId, required String nodeId, required FileAsset asset}) {
    _inflightFiles.remove(uploadId);
    _localFileUploadIds.remove(nodeId);
    _fileAssets[asset.id] = asset;
    notifyListeners();
  }

  void failFileUpload({required String uploadId, required String nodeId}) {
    _inflightFiles.remove(uploadId);
    _localFileUploadIds.remove(nodeId);
    notifyListeners();
  }

  void setInflightEmbed(String nodeId, {required bool inflight}) {
    _inflightEmbeds[nodeId] = inflight;
    notifyListeners();
  }

  void removeInflightEmbed(String nodeId) {
    _inflightEmbeds.remove(nodeId);
    notifyListeners();
  }

  void addEmbedAsset(String id, EmbedAsset asset) {
    _embedAssets[id] = asset;
    notifyListeners();
  }

  void addArchivedAsset(String id, ArchivedAsset asset) {
    _archivedAssets[id] = asset;
    notifyListeners();
  }

  void completeEmbedUnfurl({required String nodeId, required EmbedAsset asset}) {
    _inflightEmbeds.remove(nodeId);
    _embedAssets[asset.id] = asset;
    notifyListeners();
  }

  void failEmbedUnfurl({required String nodeId}) {
    _inflightEmbeds.remove(nodeId);
    notifyListeners();
  }
}
