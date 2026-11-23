//! Inspired by [`std::collections::linked_list::LinkedList`]

pub struct IntoIter<T> {
    node: Option<Box<Node<T>>>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.node.take().map(|node| {
            let Node { parent, element } = node;
            self.node = parent;
            element
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.node.is_some().into(), None)
    }
}

#[derive(Clone)]
pub struct Iter<'a, T> {
    node: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.node.take().map(|node| {
            let Node { parent, element } = node;
            self.node = parent.as_deref();
            element
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.node.is_some().into(), None)
    }
}

pub struct IterMut<'a, T> {
    node: Option<&'a mut Node<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.node.take().map(|node| {
            let Node { parent, element } = node;
            self.node = parent.as_deref_mut();
            element
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.node.is_some().into(), None)
    }
}

/// Singly-linked list that only supports push/pop and top access
#[derive(Clone)]
pub struct LinkedStack<T> {
    top: Option<Box<Node<T>>>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for LinkedStack<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Default for LinkedStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedStack<T> {
    pub const fn new() -> Self {
        Self { top: None }
    }

    pub fn top(&self) -> Option<&T> {
        self.top.as_ref().map(|node| &node.element)
    }

    pub fn top_mut(&mut self) -> Option<&mut T> {
        self.top.as_mut().map(|node| &mut node.element)
    }

    pub fn push(&mut self, value: T) {
        _ = self.push_mut(value);
    }

    pub fn push_mut(&mut self, value: T) -> &mut T {
        let parent = self.top.take();
        &mut self
            .top
            .insert(Box::new(Node {
                parent,
                element: value,
            }))
            .element
    }

    pub fn pop(&mut self) -> Option<T> {
        self.top.take().map(|node| {
            let Node { parent, element } = *node;
            self.top = parent;
            element
        })
    }

    pub const fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }
}

impl<T> IntoIterator for LinkedStack<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { node: self.top }
    }
}

impl<'a, T> IntoIterator for &'a LinkedStack<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            node: self.top.as_deref(),
        }
    }
}

impl<'a, T> IntoIterator for &'a mut LinkedStack<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            node: self.top.as_deref_mut(),
        }
    }
}

#[derive(Clone)]
struct Node<T> {
    parent: Option<Box<Self>>,
    element: T,
}

impl<T: Default> Default for Node<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Node<T> {
    const fn new(element: T) -> Self {
        Self {
            parent: None,
            element,
        }
    }
}
