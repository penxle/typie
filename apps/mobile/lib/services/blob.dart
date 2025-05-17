import 'package:cross_file/cross_file.dart';
import 'package:dio/dio.dart';
import 'package:injectable/injectable.dart';
import 'package:mime/mime.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/services/__generated__/blob_issue_blob_upload_url_mutaton.req.gql.dart';

@singleton
class Blob {
  Blob(this._client, this._dio);

  final GraphQLClient _client;
  final Dio _dio;

  Future<String> upload(XFile file) async {
    final result = await _client.request(
      GBlob_IssueBlobUploadUrl_MutationReq((b) {
        b.vars.input.filename = file.name;
      }),
    );

    final url = result.issueBlobUploadUrl.url;
    final fields = result.issueBlobUploadUrl.fields;

    final bytes = await file.readAsBytes();
    final mimeType =
        lookupMimeType(file.path, headerBytes: bytes.take(defaultMagicNumbersMaxLength).toList()) ??
        'application/octet-stream';

    final formData =
        FormData()
          ..fields.addAll(fields.asMap.entries.map((e) => MapEntry(e.key as String, e.value as String)))
          ..fields.add(MapEntry('Content-Type', mimeType))
          ..files.add(MapEntry('file', MultipartFile.fromBytes(bytes)));

    await _dio.post<void>(url, data: formData);

    return result.issueBlobUploadUrl.path;
  }
}
