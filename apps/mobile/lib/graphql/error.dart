import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:gql_exec/gql_exec.dart' as gql show GraphQLError;

part 'error.freezed.dart';

@freezed
sealed class OperationError with _$OperationError implements Exception {
  const factory OperationError.exception(Object exception) = ExceptionError;
  factory OperationError.graphql(gql.GraphQLError error) = GraphQLError;
}

@freezed
// ignore: avoid_implementing_value_types freezed
sealed class GraphQLError with _$GraphQLError implements OperationError {
  factory GraphQLError(gql.GraphQLError error) {
    return switch (error.extensions?['type']) {
      'UnexpectedError' => GraphQLError.unexpected(
        message: error.message,
        eventId: error.extensions?['eventId'] as String?,
        originalError: error.extensions?['originalError'],
      ),
      'TypieError' => GraphQLError.typie(code: error.extensions?['code'] as String, message: error.message),
      _ => GraphQLError._(error),
    };
  }

  const factory GraphQLError.unexpected({required String message, String? eventId, dynamic originalError}) =
      UnexpectedError;
  const factory GraphQLError.typie({required String code, String? message}) = TypieError;
  const factory GraphQLError._(gql.GraphQLError error) = _GraphQLError;
}
