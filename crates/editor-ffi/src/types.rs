use editor_macros::derive_ffi;

derive_ffi!(editor_model::NodeId);

derive_ffi!(editor_common::Axis);
derive_ffi!(editor_common::Direction);
derive_ffi!(editor_common::Movement);
derive_ffi!(editor_common::Rect);
derive_ffi!(editor_common::Size);
derive_ffi!(editor_view::Viewport);
derive_ffi!(editor_renderer::BackendKind);

derive_ffi!(editor_state::Affinity);
derive_ffi!(editor_state::Position);
derive_ffi!(editor_state::Selection);

derive_ffi!(editor_model::RootNode);
derive_ffi!(editor_model::TextAlign);
derive_ffi!(editor_model::ParagraphNode);
derive_ffi!(editor_model::BlockquoteVariant);
derive_ffi!(editor_model::BlockquoteNode);
derive_ffi!(editor_model::CalloutVariant);
derive_ffi!(editor_model::CalloutNode);
derive_ffi!(editor_model::TextNode);
derive_ffi!(editor_model::BulletListNode);
derive_ffi!(editor_model::OrderedListNode);
derive_ffi!(editor_model::ListItemNode);
derive_ffi!(editor_model::FoldNode);
derive_ffi!(editor_model::FoldTitleNode);
derive_ffi!(editor_model::FoldContentNode);
derive_ffi!(editor_model::TableBorderStyle);
derive_ffi!(editor_model::TableAlign);
derive_ffi!(editor_model::TableNode);
derive_ffi!(editor_model::TableRowNode);
derive_ffi!(editor_model::TableCellNode);
derive_ffi!(editor_model::ImageNode);
derive_ffi!(editor_model::FileNode);
derive_ffi!(editor_model::EmbedNode);
derive_ffi!(editor_model::ArchivedNode);
derive_ffi!(editor_model::HardBreakNode);
derive_ffi!(editor_model::HorizontalRuleVariant);
derive_ffi!(editor_model::HorizontalRuleNode);
derive_ffi!(editor_model::PageBreakNode);

derive_ffi!(editor_model::ModifierType);
derive_ffi!(editor_model::Modifier);
derive_ffi!(editor_model::NodeType);
derive_ffi!(editor_model::Node);

derive_ffi!(editor_core::Key);
derive_ffi!(editor_core::KeyModifiers);
derive_ffi!(editor_core::KeyEvent);
derive_ffi!(editor_core::PointerButton);
derive_ffi!(editor_core::DragPayload);
derive_ffi!(editor_core::DragEvent);
derive_ffi!(editor_core::PointerEvent);
derive_ffi!(editor_core::FontMapping);
derive_ffi!(editor_core::SystemEvent);

derive_ffi!(editor_core::BreakKind);
derive_ffi!(editor_core::InsertionIntent);
derive_ffi!(editor_core::DeletionIntent);
derive_ffi!(editor_core::FormattingIntent);
derive_ffi!(editor_core::SelectionIntent);
derive_ffi!(editor_core::TableOp);
derive_ffi!(editor_core::NodeIntent);
derive_ffi!(editor_core::ClipboardIntent);
derive_ffi!(editor_core::CompositionIntent);
derive_ffi!(editor_core::NavigationIntent);
derive_ffi!(editor_core::HistoryIntent);
derive_ffi!(editor_core::Intent);

derive_ffi!(editor_transaction::Effect);

derive_ffi!(editor_core::Message);
derive_ffi!(editor_core::StateField);
derive_ffi!(editor_core::EditorEvent);
