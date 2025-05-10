import 'package:gql_exec/gql_exec.dart';
import 'package:gql_transform_link/gql_transform_link.dart';

TransformLink authLink({required String? Function() getAccessToken}) {
  return TransformLink(
    requestTransformer: (request) {
      final accessToken = getAccessToken();

      return request.updateContextEntry<HttpLinkHeaders>(
        (headers) => HttpLinkHeaders(
          headers: {...?headers?.headers, if (accessToken != null) 'Authorization': 'Bearer $accessToken'},
        ),
      );
    },
  );
}
