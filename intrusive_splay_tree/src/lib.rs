#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![no_std]

//! An intrusive, allocation-free [splay tree] implementation.
//!
//! Splay trees are self-adjusting, meaning that operating on an element (for
//! example, doing a `find` or an `insert`) rebalances the tree in such a way
//! that the element becomes the root. This means that subsequent operations on
//! that element are *O(1)* as long as no other element is operated on in the
//! meantime.
//!
//! ## Implementation and Goals
//!
//! * **Intrusive:** The space for the subtree pointers is stored *inside* the
//! element type. In non-intrusive trees, we would have a node type that
//! contains the subtree pointers and either a pointer to the element or we
//! would move the element into the node. The intrusive design inverts the
//! relationship, so that the elements hold the subtree pointers within
//! themselves.
//!
//! * **Freedom from allocations and moves:** The intrusive design enables this
//! implementation to fully avoid both allocations and moving elements in
//! memory. Since the space for subtree pointers already exists in the element,
//! no allocation is necessary, just a handful of pointer writes. Therefore,
//! this implementation can be used in constrained environments that don't have
//! access to an allocator (e.g. some embedded devices or within a signal
//! handler) and with types that can't move in memory (e.g. `pthread_mutex_t`).
//!
//! * **Small code size:** This implementation is geared towards small code
//! size, and uses trait objects internally to avoid the code bloat induced by
//! monomorphization. This implementation is suitable for targeting WebAssembly,
//! where code is downloaded over the network, and code bloat delays Web page
//! loading.
//!
//! * **Nodes do not have parent pointers**: An intrusive node is only two words
//! in size: left and right sub tree pointers. There are no parent pointers,
//! which would require another word of overhead. To meet this goal, the
//! implementation uses the "top-down" variant of splay trees.
//!
//! [splay tree]: https://en.wikipedia.org/wiki/Splay_tree
//! [paper]: http://www.cs.cmu.edu/~sleator/papers/self-adjusting.pdf
//!
//! ## Constraints
//!
//! * **Elements within a tree must all have the same lifetime.** This means
//! that you must use something like the [`typed_arena`][arena] crate for
//! allocation, or be working with static data, etc.
//!
//! * **Elements in an intrusive collections are inherently shared.** They are
//! always potentially aliased by the collection(s) they are in. In the other
//! direction, a particular intrusive collection only has a shared reference to
//! the element, since elements can both be in many intrusive collections at the
//! same time. Therefore, you cannot get a unique, mutable reference to an
//! element out of an intrusive splay tree. To work around this, you may need to
//! liberally use interior mutability, for example by leveraging `Cell`,
//! `RefCell`, and `Mutex`.
//!
//! [arena]: https://crates.io/crates/typed-arena
//!
//! ## Example
//!
//! This example defines a `Monster` type, where each of its instances live
//! within two intrusive trees: one ordering monsters by their name, and the
//! other ordering them by their health.
//!
//! ```
//! #[macro_use]
//! extern crate intrusive_splay_tree;
//! extern crate typed_arena;
//!
//! use intrusive_splay_tree::SplayTree;
//!
//! use std::cmp::Ordering;
//! use std::marker::PhantomData;
//!
//! // We have a monster type, and we want to query monsters by both name and
//! // health.
//! #[derive(Debug)]
//! struct Monster<'a> {
//!     name: String,
//!     health: u64,
//!
//!     // An intrusive node so we can put monsters in a tree to query by name.
//!     by_name_node: intrusive_splay_tree::Node<'a>,
//!
//!     // Another intrusive node so we can put monsters in a second tree (at
//!     // the same time!) and query them by health.
//!     by_health_node: intrusive_splay_tree::Node<'a>,
//! }
//!
//! // Define a type for trees where monsters are ordered by name.
//! struct MonstersByName;
//!
//! // Implement `IntrusiveNode` for the `MonstersByName` tree, where the
//! // element type is `Monster` and the field in `Monster` that has this tree's
//! // intrusive node is `by_name`.
//! impl_intrusive_node! {
//!     impl<'a> IntrusiveNode<'a> for MonstersByName
//!     where
//!         type Elem = Monster<'a>,
//!         node = by_name_node;
//! }
//!
//! // Define how to order `Monster`s within the `MonstersByName` tree by
//! // implementing `TreeOrd`.
//! impl<'a> intrusive_splay_tree::TreeOrd<'a, MonstersByName> for Monster<'a> {
//!     fn tree_cmp(&self, rhs: &Monster<'a>) -> Ordering {
//!         self.name.cmp(&rhs.name)
//!     }
//! }
//!
//! // And do all the same things for trees where monsters are ordered by health...
//! struct MonstersByHealth;
//! impl_intrusive_node! {
//!     impl<'a> IntrusiveNode<'a> for MonstersByHealth
//!     where
//!         type Elem = Monster<'a>,
//!         node = by_health_node;
//! }
//! impl<'a> intrusive_splay_tree::TreeOrd<'a, MonstersByHealth> for Monster<'a> {
//!     fn tree_cmp(&self, rhs: &Monster<'a>) -> Ordering {
//!         self.health.cmp(&rhs.health)
//!     }
//! }
//!
//! // We can also implement `TreeOrd` for other types, so that we can query the
//! // tree by these types. For example, we want to query the `MonstersByHealth`
//! // tree by some `u64` health value, and we want to query the `MonstersByName`
//! // tree by some `&str` name value.
//!
//! impl<'a> intrusive_splay_tree::TreeOrd<'a, MonstersByHealth> for u64 {
//!     fn tree_cmp(&self, rhs: &Monster<'a>) -> Ordering {
//!         self.cmp(&rhs.health)
//!     }
//! }
//!
//! impl<'a> intrusive_splay_tree::TreeOrd<'a, MonstersByName> for str {
//!     fn tree_cmp(&self, rhs: &Monster<'a>) -> Ordering {
//!         self.cmp(&rhs.name)
//!     }
//! }
//!
//! impl<'a> Monster<'a> {
//!     /// The `Monster` constructor allocates `Monster`s in a typed arena, and
//!     /// inserts the new `Monster` in both trees.
//!     pub fn new(
//!         arena: &'a typed_arena::Arena<Monster<'a>>,
//!         name: String,
//!         health: u64,
//!         by_name_tree: &mut SplayTree<'a, MonstersByName>,
//!         by_health_tree: &mut SplayTree<'a, MonstersByHealth>
//!     ) -> &'a Monster<'a> {
//!         let monster = arena.alloc(Monster {
//!             name,
//!             health,
//!             by_name_node: Default::default(),
//!             by_health_node: Default::default(),
//!         });
//!
//!         by_name_tree.insert(monster);
//!         by_health_tree.insert(monster);
//!
//!         monster
//!     }
//! }
//!
//! fn main() {
//!     // The arena that the monsters will live within.
//!     let mut arena = typed_arena::Arena::new();
//!
//!     // The splay trees ordered by name and health respectively.
//!     let mut by_name_tree = SplayTree::default();
//!     let mut by_health_tree = SplayTree::default();
//!
//!     // Now let's create some monsters, inserting them into the trees!
//!
//!     Monster::new(
//!         &arena,
//!         "Frankenstein's Monster".into(),
//!         99,
//!         &mut by_name_tree,
//!         &mut by_health_tree,
//!     );
//!
//!     Monster::new(
//!         &arena,
//!         "Godzilla".into(),
//!         2000,
//!         &mut by_name_tree,
//!         &mut by_health_tree,
//!     );
//!
//!     Monster::new(
//!         &arena,
//!         "Vegeta".into(),
//!         9001,
//!         &mut by_name_tree,
//!         &mut by_health_tree,
//!     );
//!
//!     // Query the `MonstersByName` tree by a name.
//!
//!     let godzilla = by_name_tree.find("Godzilla").unwrap();
//!     assert_eq!(godzilla.name, "Godzilla");
//!
//!     assert!(by_name_tree.find("Gill-Man").is_none());
//!
//!     // Query the `MonstersByHealth` tree by a health.
//!
//!     let vegeta = by_health_tree.find(&9001).unwrap();
//!     assert_eq!(vegeta.name, "Vegeta");
//!
//!     assert!(by_health_tree.find(&0).is_none());
//! }
//! ```

extern crate unreachable;

mod internal;
mod node;

pub use node::Node;

use core::cmp;
use core::fmt;
use core::iter;
use core::marker::PhantomData;

/// Defines how to get the intrusive node from a particular kind of
/// `SplayTree`'s element type.
///
/// Don't implement this by hand -- doing so is both boring and dangerous!
/// Instead, use the `impl_intrusive_node!` macro.
pub unsafe trait IntrusiveNode<'a>
where
    Self: Sized,
{
    /// The element struct type that contains a node for this tree.
    type Elem: TreeOrd<'a, Self>;

    /// Get the node for this tree from the given element.
    fn elem_to_node(&'a Self::Elem) -> &'a Node<'a>;

    /// Get the element for this node (by essentially doing `offsetof` the
    /// node's field).
    ///
    /// ## Safety
    ///
    /// Given a node inside a different element type, or a node for a different
    /// tree within the same element type, this method will result in memory
    /// unsafety.
    unsafe fn node_to_elem(&'a Node<'a>) -> &'a Self::Elem;
}

/// Implement `IntrusiveNode` for a particular kind of `SplayTree` and its
/// element type.
#[macro_export]
macro_rules! impl_intrusive_node {
    (
        impl< $($typarams:tt),* >
            IntrusiveNode<$intrusive_node_lifetime:tt>
            for $tree:ty
        where
            type Elem = $elem:ty ,
            node = $node:ident ;
    ) => {
        unsafe impl< $( $typarams )* > $crate::IntrusiveNode<$intrusive_node_lifetime> for $tree {
            type Elem = $elem;

            fn elem_to_node(
                elem: & $intrusive_node_lifetime Self::Elem
            ) -> & $intrusive_node_lifetime $crate::Node< $intrusive_node_lifetime > {
                &elem. $node
            }

            unsafe fn node_to_elem(
                node: & $intrusive_node_lifetime $crate::Node< $intrusive_node_lifetime >
            ) -> & $intrusive_node_lifetime Self::Elem {
                let s: Self::Elem = $crate::uninitialized();

                let offset = {
                    let base = &s as *const _ as usize;

                    // XXX: We are careful not to deref the uninitialized data
                    // by using irrefutable let patterns instead of `s.$node`.
                    let Self::Elem { ref $node, .. } = s;

                    // Annotate with explicit types here so that compilation
                    // will fail if someone uses this macro with a non-Node
                    // field of `Self::Elem`.
                    let $node: &$crate::Node = $node;
                    let field = $node as *const $crate::Node as usize;

                    field - base
                };

                // Don't run destructors on uninitialized data.
                $crate::forget(s);

                let node = node as *const _ as *const u8;
                let elem = node.offset(-(offset as isize)) as *const Self::Elem;
                &*elem
            }
        }
    }
}

#[doc(hidden)]
#[inline(always)]
pub unsafe fn uninitialized<T>() -> T {
    core::mem::uninitialized()
}

#[doc(hidden)]
#[inline(always)]
pub unsafe fn forget<T>(t: T) {
    core::mem::forget(t);
}

/// A total ordering between the `Self` type and the tree's element type
/// `T::Elem`.
///
/// Different from `Ord` in that it allows `Self` and `T::Elem` to be distinct
/// types, so that you can query a splay tree without fully constructing its
/// element type.
pub trait TreeOrd<'a, T: IntrusiveNode<'a>> {
    /// What is the ordering relationship between `self` and the given tree
    /// element?
    fn tree_cmp(&self, elem: &'a T::Elem) -> cmp::Ordering;
}

struct Query<'a, 'b, K, T>
where
    T: 'a + IntrusiveNode<'a>,
    K: 'b + ?Sized + TreeOrd<'a, T>,
{
    key: &'b K,
    _phantom: PhantomData<&'a T>,
}

impl<'a, 'b, K, T> Query<'a, 'b, K, T>
where
    T: IntrusiveNode<'a>,
    K: 'b + ?Sized + TreeOrd<'a, T>,
{
    #[inline]
    fn new(key: &'b K) -> Query<'a, 'b, K, T> {
        Query {
            key,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'b, K, T> internal::CompareToNode<'a> for Query<'a, 'b, K, T>
where
    T: 'a + IntrusiveNode<'a>,
    T::Elem: 'a,
    K: 'b + ?Sized + TreeOrd<'a, T>,
{
    #[inline]
    unsafe fn compare_to_node(&self, node: &'a Node<'a>) -> cmp::Ordering {
        let val = T::node_to_elem(node);
        self.key.tree_cmp(val)
    }
}

/// An intrusive splay tree.
///
/// The tree is parameterized by some marker type `T` whose `IntrusiveNode`
/// implementation defines:
///
/// * the element type contained in this tree: `T::Elem`,
/// * how to get the intrusive node for this tree within an element,
/// * and how to get the container element from a given intrusive node for this
/// tree.
pub struct SplayTree<'a, T>
where
    T: IntrusiveNode<'a>,
    T::Elem: 'a,
{
    tree: internal::SplayTree<'a>,
    _phantom: PhantomData<&'a T::Elem>,
}

impl<'a, T> Default for SplayTree<'a, T>
where
    T: 'a + IntrusiveNode<'a>,
    T::Elem: 'a,
{
    #[inline]
    fn default() -> SplayTree<'a, T> {
        SplayTree {
            tree: internal::SplayTree::default(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> fmt::Debug for SplayTree<'a, T>
where
    T: 'a + IntrusiveNode<'a>,
    T::Elem: 'a + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let set = &mut f.debug_set();
        self.walk(|x| {
            set.entry(x);
        });
        set.finish()
    }
}

impl<'a, T> Extend<&'a T::Elem> for SplayTree<'a, T>
where
    T: 'a + IntrusiveNode<'a>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T::Elem>>(&mut self, iter: I) {
        for x in iter {
            self.insert(x);
        }
    }
}

impl<'a, T> iter::FromIterator<&'a T::Elem> for SplayTree<'a, T>
where
    T: 'a + IntrusiveNode<'a>,
    T::Elem: fmt::Debug,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a T::Elem>>(iter: I) -> Self {
        let mut me = SplayTree::default();
        me.extend(iter);
        me
    }
}

impl<'a, T> SplayTree<'a, T>
where
    T: 'a + IntrusiveNode<'a>,
{
    /// Is this tree empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Get a reference to the root element, if any exists.
    pub fn root(&self) -> Option<&'a T::Elem> {
        self.tree.root().map(|r| unsafe { T::node_to_elem(r) })
    }

    /// Find an element in the tree.
    ///
    /// This operation will splay the queried element to the root of the tree.
    ///
    /// The `key` must be of a type that implements `TreeOrd` for this tree's
    /// `T` type. The element type `T::Elem` must always implement `TreeOrd<T>`,
    /// so you can search the tree by element. You can also implement
    /// `TreeOrd<T>` for additional key types. This allows you to search the
    /// tree without constructing a full element.
    #[inline]
    pub fn find<K>(&mut self, key: &K) -> Option<&'a T::Elem>
    where
        K: ?Sized + TreeOrd<'a, T>,
    {
        unsafe {
            let query: Query<_, T> = Query::new(key);
            self.tree.find(&query).map(|node| T::node_to_elem(node))
        }
    }

    /// Insert a new element into this tree.
    ///
    /// Returns `true` if the element was inserted into the tree.
    ///
    /// Returns `false` if there was already an element in the tree for which
    /// `TreeOrd` returned `Ordering::Equal`. In this case, the extant element
    /// is left in the tree, and `elem` is not inserted.
    ///
    /// This operation will splay the inserted element to the root of the tree.
    ///
    /// It is a logic error to insert an element that is already inserted in a
    /// `T` tree.
    ///
    /// ## Panics
    ///
    /// If `debug_assertions` are enabled, then this function may panic if
    /// `elem` is already in a `T` tree. If `debug_assertions` are not defined,
    /// the behavior is safe, but unspecified.
    #[inline]
    pub fn insert(&mut self, elem: &'a T::Elem) -> bool {
        unsafe {
            let query: Query<_, T> = Query::new(elem);
            let node = T::elem_to_node(elem);
            self.tree.insert(&query, node)
        }
    }

    /// Find and remove an element from the tree.
    ///
    /// If a matching element is found and removed, then `Some(removed_element)`
    /// is returned. Otherwise `None` is returned.
    ///
    /// The `key` must be of a type that implements `TreeOrd` for this tree's
    /// `T` type. The element type `T::Elem` must always implement `TreeOrd<T>`,
    /// so you can remove an element directly. You can also implement
    /// `TreeOrd<T>` for additional key types. This allows you to search the
    /// tree without constructing a full element, and remove the element that
    /// matches the given key, if any.
    #[inline]
    pub fn remove<K>(&mut self, key: &K) -> Option<&'a T::Elem>
    where
        K: ?Sized + TreeOrd<'a, T>,
    {
        unsafe {
            let query: Query<_, T> = Query::new(key);
            self.tree.remove(&query).map(|node| T::node_to_elem(node))
        }
    }

    /// Walk the tree in order.
    ///
    /// The `C` type controls whether iteration should continue, or break and
    /// return a `C::Result` value. You can use `()` as `C`, and that always
    /// continues iteration. Using `Result<(), E>` as `C` allows you to halt
    /// iteration on error, and propagate the error value. Using `Option<T>` as
    /// `C` allows you to search for some value, halt iteration when its found,
    /// and return it.
    #[inline]
    pub fn walk<F, C>(&self, mut f: F) -> Option<C::Result>
    where
        F: FnMut(&'a T::Elem) -> C,
        C: WalkControl,
    {
        let mut result = None;
        self.tree.walk(&mut |node| unsafe {
            let elem = T::node_to_elem(node);
            result = f(elem).should_break();
            result.is_none()
        });
        result
    }
}

/// A trait that guides whether `SplayTree::walk` should continue or break, and
/// what the return value is.
pub trait WalkControl {
    /// The result type that is returned when we break.
    type Result;

    /// If iteration should halt, return `Some`. If iteration should continue,
    /// return `None`.
    fn should_break(self) -> Option<Self::Result>;
}

impl WalkControl for () {
    type Result = ();

    fn should_break(self) -> Option<()> {
        None
    }
}

impl<T> WalkControl for Option<T> {
    type Result = T;

    fn should_break(mut self) -> Option<T> {
        self.take()
    }
}

impl<E> WalkControl for Result<(), E> {
    type Result = E;

    fn should_break(self) -> Option<E> {
        self.err()
    }
}
