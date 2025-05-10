import 'dart:io';

import 'package:gql_exec/gql_exec.dart';
import 'package:gql_transform_link/gql_transform_link.dart';

TransformLink cookieLink({required void Function(Cookie cookie) setter}) {
  return TransformLink(
    responseTransformer: (response) {
      final context = response.context.entry<HttpLinkResponseContext>();
      final headers = context?.rawHeaders?['set-cookie'];

      if (headers != null) {
        for (final header in headers) {
          final cookie = Cookie.fromSetCookieValue(header);
          setter(cookie);
        }
      }

      return response;
    },
  );
}
