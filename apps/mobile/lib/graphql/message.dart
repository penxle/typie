import 'dart:io';
import 'dart:isolate';

import 'package:freezed_annotation/freezed_annotation.dart';

part 'message.freezed.dart';

@freezed
sealed class GraphQLMessage with _$GraphQLMessage {
  const factory GraphQLMessage.port(SendPort port) = GraphQLPortMessage;
  const factory GraphQLMessage.cookie(Cookie cookie) = GraphQLCookieMessage;
  const factory GraphQLMessage.accessToken(String? token) = GraphQLAccessTokenMessage;
}
