final _pendingAutoDiscardSlugs = <String>{};

class AutoDiscardSession {
  AutoDiscardSession._(this._enabled);

  factory AutoDiscardSession.consume(String slug) {
    return AutoDiscardSession._(_pendingAutoDiscardSlugs.remove(slug));
  }

  final bool _enabled;
  bool _edited = false;
  bool _consumed = false;

  void markEdited() {
    _edited = true;
  }

  bool takeShouldDeleteOnClose() {
    if (_consumed) {
      return false;
    }
    _consumed = true;
    return _enabled && !_edited;
  }
}

void markAutoDiscardCandidate(String slug) {
  _pendingAutoDiscardSlugs.add(slug);
}
