//! An explicit terminal destination, independent of the daemon's current focus.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputTarget {
    Pane(String),
    Popup(String),
}
