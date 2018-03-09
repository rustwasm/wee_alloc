use core::cell::Cell;
use core::fmt;

/// A splay tree node that is embedded within some container type.
///
/// The container type may have multiple `Node` members to fit into multiple
/// `SplayTree`s. For example, if you had a set of memory blocks and wanted to
/// query them by either size or alignment. You could have two intrusive
/// `SplayTree`s, one sorted by size and the other by alignment:
///
/// ```
/// struct Monster<'a> {
///     // Intrusive node for splay tree sorted by name.
///     by_name: intrusive_splay_tree::Node<'a>,
///
///     // Intrusive node for splay tree sorted by health.
///     by_health: intrusive_splay_tree::Node<'a>,
///
///     // The monster's name.
///     name: String,
///
///     // The monsters health.
///     health: usize,
/// }
/// ```
pub struct Node<'a> {
    pub(crate) left: Cell<Option<&'a Node<'a>>>,
    pub(crate) right: Cell<Option<&'a Node<'a>>>,
}

impl<'a> Default for Node<'a> {
    #[inline]
    fn default() -> Node<'a> {
        Node {
            left: Cell::new(None),
            right: Cell::new(None),
        }
    }
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Node").finish()
    }
}

impl<'a> Node<'a> {
    /// Get this node's left subtree.
    ///
    /// This is a low-level API, and should only be used for custom tree walking
    /// and searching, for example to implement a custom pre-order traversal.
    ///
    /// Use the unsafe `IntrusiveNode::node_to_elem` method to convert the
    /// resulting `Node` reference into a reference to its container element
    /// type.
    pub fn left(&self) -> Option<&'a Node> {
        self.left.get()
    }

    /// Get this node's right subtree.
    ///
    /// This is a low-level API, and should only be used for custom tree walking
    /// and searching, for example to implement a custom pre-order traversal.
    ///
    /// Use the unsafe `IntrusiveNode::node_to_elem` method to convert the
    /// resulting `Node` reference into a reference to its container element
    /// type.
    pub fn right(&self) -> Option<&'a Node> {
        self.right.get()
    }

    pub(crate) fn walk(&'a self, f: &mut FnMut(&'a Node<'a>) -> bool) -> bool {
        if let Some(left) = self.left.get() {
            if !left.walk(f) {
                return false;
            }
        }

        if !f(self) {
            return false;
        }

        if let Some(right) = self.right.get() {
            if !right.walk(f) {
                return false;
            }
        }

        true
    }
}
