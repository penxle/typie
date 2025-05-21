import 'dart:io';

import 'package:collection/collection.dart';
import 'package:dio/dio.dart';
import 'package:injectable/injectable.dart';
import 'package:mime/mime.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/services/__generated__/blob.req.gql.dart';

@singleton
class Blob {
  Blob(this._client, this._dio);

  final GraphQLClient _client;
  final Dio _dio;

  Future<String> mime(File file) async {
    final headerBytes = (await file.openRead(0, defaultMagicNumbersMaxLength).toList()).flattenedToList;
    return lookupMimeType(file.path, headerBytes: headerBytes) ?? 'application/octet-stream';
  }

  Future<String> upload(File file) async {
    final result = await _client.request(
      GBlob_IssueBlobUploadUrl_MutationReq((b) => b..vars.input.filename = file.uri.pathSegments.last),
    );

    final url = result.issueBlobUploadUrl.url;
    final fields = result.issueBlobUploadUrl.fields;

    final mimeType = await mime(file);
    final stream = file.openRead();
    final length = await file.length();

    final formData = FormData()
      ..fields.addAll(fields.asMap.entries.map((e) => MapEntry(e.key as String, e.value as String)))
      ..fields.add(MapEntry('Content-Type', mimeType))
      ..files.add(MapEntry('file', MultipartFile.fromStream(() => stream, length)));

    await _dio.post<void>(url, data: formData);

    return result.issueBlobUploadUrl.path;
  }
}
