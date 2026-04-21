import 'dart:io';
import 'dart:typed_data';

import 'package:collection/collection.dart';
import 'package:dio/dio.dart';
import 'package:injectable/injectable.dart';
import 'package:mime/mime.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/services/__generated__/issue_blob_upload_url_mutation.req.gql.dart';

@singleton
class Blob {
  Blob(this._client, this._dio);

  final GraphQLClient _client;
  final Dio _dio;

  Future<String> mime(File file) async {
    final headerBytes = (await file.openRead(0, defaultMagicNumbersMaxLength).toList()).flattenedToList;
    return lookupMimeType(file.path, headerBytes: headerBytes) ?? 'application/octet-stream';
  }

  Future<String> upload(File file, {String? filename}) async {
    final uploadName = filename ?? file.uri.pathSegments.last;
    final mimeType = await mime(file);
    final stream = file.openRead();
    final length = await file.length();
    return _uploadMultipart(
      filename: uploadName,
      mimeType: mimeType,
      multipartFile: MultipartFile.fromStream(() => stream, length),
    );
  }

  Future<String> uploadBytes(Uint8List bytes, {required String filename, String? mimeType}) {
    final resolvedMimeType = mimeType ?? lookupMimeType(filename, headerBytes: bytes) ?? 'application/octet-stream';
    return _uploadMultipart(
      filename: filename,
      mimeType: resolvedMimeType,
      multipartFile: MultipartFile.fromBytes(bytes, filename: filename),
    );
  }

  Future<String> _uploadMultipart({
    required String filename,
    required String mimeType,
    required MultipartFile multipartFile,
  }) async {
    final result = await _client.request(
      GBlob_IssueBlobUploadUrl_MutationReq((b) => b..vars.input.filename = filename),
    );

    final url = result.issueBlobUploadUrl.url;
    final fields = result.issueBlobUploadUrl.fields;

    final formData = FormData()
      ..fields.addAll(fields.asMap.entries.map((e) => MapEntry(e.key as String, e.value as String)))
      ..fields.add(MapEntry('Content-Type', mimeType))
      ..files.add(MapEntry('file', multipartFile));

    await _dio.post<void>(url, data: formData);

    return result.issueBlobUploadUrl.path;
  }
}
