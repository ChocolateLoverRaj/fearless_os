use core::fmt::Debug;

use alloc::boxed::Box;

pub struct LinkedListNode<T> {
    pub data: T,
    pub next: Option<Box<Self>>,
}

impl<T> LinkedListNode<T> {
    pub const fn new(data: T) -> Self {
        Self { data, next: None }
    }
}

pub struct LinkedList<T> {
    pub start: Option<Box<LinkedListNode<T>>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self { start: None }
    }

    pub fn push_back(&mut self, data: T) {
        self.push_back_boxed(Box::new(LinkedListNode { data, next: None }));
    }

    pub fn push_back_boxed(&mut self, node: Box<LinkedListNode<T>>) {
        let mut current = &mut self.start;
        while let Some(next) = current {
            current = &mut next.next;
        }
        *current = Some(node);
    }
}

impl<T: Ord> LinkedList<T> {
    pub fn insert_sorted_boxed(&mut self, mut node: Box<LinkedListNode<T>>) {
        let mut ptr = &mut self.start;
        loop {
            if ptr.is_some() {
                if node.data < ptr.as_ref().unwrap().data {
                    break;
                }
                ptr = &mut ptr.as_mut().unwrap().next;
            } else {
                break;
            }
        }
        node.next = ptr.take();
        *ptr = Some(node);
    }
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> {
    type IntoIter = LinkedListIterator<'a, T>;
    type Item = &'a LinkedListNode<T>;

    fn into_iter(self) -> Self::IntoIter {
        LinkedListIterator {
            node: self.start.as_deref(),
        }
    }
}

pub struct LinkedListIterator<'a, T> {
    node: Option<&'a LinkedListNode<T>>,
}

impl<'a, T: 'a> Iterator for LinkedListIterator<'a, T> {
    // TODO: consider returning just &'a T and abstract away interal fields and make them private.
    type Item = &'a LinkedListNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.node?;
        self.node = node.next.as_deref();
        Some(node)
    }
}

impl<T: Debug> Debug for LinkedList<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug_list = f.debug_list();
        for item in self {
            debug_list.entry(&item.data);
        }
        debug_list.finish()
    }
}
