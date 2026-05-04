use serde::{Deserialize, Serialize};

use crate::{Dot, Op};

/// Wire message between client and server. Variant meaning is determined by
/// sender/receiver role:
///
/// - Client → Server `Ops`: ops to receive into server's OpGraph + broadcast.
/// - Server → Client `Ops`: ops from missing-response or broadcast to apply.
/// - Client → Server `Dots`: client_heads for missing-request.
/// - Server → Client `Dots`: ack — client removes from its pending-push set.
/// - Server → Client `ResendRequest`: explicit negative-ack — server holds
///   no record of these dots; client must re-send them with full local
///   ancestry. Distinct from `Dots` so the client never confuses a
///   repeated ack with a recovery request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMessage<P> {
    Ops(Vec<Op<P>>),
    Dots(Vec<Dot>),
    ResendRequest(Vec<Dot>),
}
