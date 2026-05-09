use serde::{Deserialize, Serialize};

use crate::Dot;

/// Wire message between client and server. Variant meaning is determined by
/// sender/receiver role:
///
/// - Client → Server `Changesets`: changesets to receive into server's
///   OpGraph + broadcast.
/// - Server → Client `Changesets`: changesets from missing-response or
///   broadcast to apply. Receivers either accept the full `Changeset` or
///   reject it — atomicity is per-changeset.
/// - Client → Server `Dots`: client_heads for missing-request.
/// - Server → Client `Dots`: ack — client removes from its pending-push set.
/// - Server → Client `ResendRequest`: explicit negative-ack — server holds
///   no record of these dots; client must re-send them with full local
///   ancestry. Distinct from `Dots` so the client never confuses a
///   repeated ack with a recovery request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMessage<P> {
    Changesets(Vec<crate::Changeset<P>>),
    Dots(Vec<Dot>),
    ResendRequest(Vec<Dot>),
}
