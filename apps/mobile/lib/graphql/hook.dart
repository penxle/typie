import 'package:ferry/ferry.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';

TData? useQuery<TData, TVars>(OperationRequest<TData, TVars> operation) {
  final client = useService<GraphQLClient>();

  final stream = useMemoized(() => client.raw.request(operation).distinct(), [operation]);
  final snapshot = useStream(stream);

  if (snapshot.data?.hasErrors ?? false) {
    // ignore: only_throw_errors -- it is an error
    throw snapshot.data!.linkException ?? snapshot.data!.graphqlErrors!.first;
  }

  return snapshot.data?.data;
}
