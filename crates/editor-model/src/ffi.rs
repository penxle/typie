#[cfg(feature = "uniffi")]
type NodeIdVec = imbl::Vector<crate::id::NodeId>;

#[cfg(feature = "uniffi")]
type NodeEntryHashMap = imbl::HashMap<crate::id::NodeId, crate::entry::NodeEntry>;

#[cfg(feature = "uniffi")]
::uniffi::custom_type!(NodeIdVec, Vec<crate::id::NodeId>, {
    remote,
    lower: |obj| obj.into_iter().collect(),
    try_lift: |val| Ok(val.into_iter().collect()),
});

#[cfg(feature = "uniffi")]
::uniffi::custom_type!(NodeEntryHashMap, std::collections::HashMap<crate::id::NodeId, crate::entry::NodeEntry>, {
    remote,
    lower: |obj| obj.into_iter().collect(),
    try_lift: |val| Ok(val.into_iter().collect()),
});
