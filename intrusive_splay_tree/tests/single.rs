extern crate intrusive_splay_tree;

use intrusive_splay_tree::{IntrusiveNode, Node, TreeOrd};
use std::cmp::Ordering;
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Single<'a> {
    pub value: usize,
    node: Node<'a>,
}

impl<'a> Single<'a> {
    pub fn new(x: usize) -> Single<'a> {
        Single {
            value: x,
            node: Default::default(),
        }
    }
}

pub struct SingleTree<'a>(PhantomData<&'a Single<'a>>);

unsafe impl<'a> IntrusiveNode<'a> for SingleTree<'a> {
    type Elem = Single<'a>;

    fn elem_to_node(elem: &'a Self::Elem) -> &'a Node<'a> {
        &elem.node
    }

    unsafe fn node_to_elem(node: &'a Node<'a>) -> &'a Self::Elem {
        let offset = {
            let c = Single::default();
            let node = &c.node as *const _ as usize;
            let c = &c as *const _ as usize;
            node - c
        };
        let node = node as *const _ as *const u8;
        let elem = node.offset(-(offset as isize)) as *const Self::Elem;
        &*elem
    }
}

impl<'a> TreeOrd<'a, SingleTree<'a>> for Single<'a> {
    fn tree_cmp(&self, rhs: &Single<'a>) -> Ordering {
        self.value.cmp(&rhs.value)
    }
}

impl<'a> TreeOrd<'a, SingleTree<'a>> for usize {
    fn tree_cmp(&self, rhs: &Single<'a>) -> Ordering {
        self.cmp(&rhs.value)
    }
}
