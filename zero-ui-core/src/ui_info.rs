// TODO, 
//
// 1 - FrameInfo -> UiTreeInfo
// 2 - Rebuild UiTree just after update that requested it.
// 3 - Use shared reference to update outer/inner sizes so we don't need to rebuild after every layout.
//
// Extensions that query the Ui tree must check a flag on the `AppExtension::update` to response to UiTree changes
// in the same update pass, windows must rebuild info when an update requests it and set this flag in the `AppExtension::update_ui` pass.
//
// Alternatively could be an event?