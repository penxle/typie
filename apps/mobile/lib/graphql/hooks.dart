import 'dart:async';

import 'package:auto_route/auto_route.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';

TData? useQuery<TData, TVars>(OperationRequest<TData, TVars> operation) {
  final client = useService<GraphQLClient>();

  final stream = useMemoized(() => client.raw.request(operation).distinct(), [operation]);
  final snapshot = useStream(stream);

  useRefetchOnRouteResumed(operation);

  if (snapshot.data?.hasErrors ?? false) {
    // ignore: only_throw_errors -- it is an error
    throw snapshot.data!.linkException ?? snapshot.data!.graphqlErrors!.first;
  }

  return snapshot.data?.data;
}

void useRefetchOnRouteResumed<TData, TVars>(OperationRequest<TData, TVars> operation) {
  final client = useService<GraphQLClient>();
  final context = useContext();
  final scope = context.dependOnInheritedWidgetOfExactType<RouteDataScope>();
  final observer = scope != null ? RouterScope.of(context).firstObserverOfType<AutoRouteObserver>() : null;
  final routeData = scope?.routeData;

  useEffect(() {
    if (observer == null || routeData == null) {
      return null;
    }

    final aware = _RefetchRouteAware(
      onResume: () {
        unawaited(client.refetch(operation));
      },
    );

    observer.subscribe(aware, routeData);
    return () => observer.unsubscribe(aware);
  }, [observer, routeData, operation]);
}

class _RefetchRouteAware with AutoRouteAware {
  _RefetchRouteAware({required this.onResume});
  final VoidCallback onResume;

  @override
  void didPopNext() => onResume();

  @override
  void didChangeTabRoute(TabPageRoute previousRoute) => onResume();
}
