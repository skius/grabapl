use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BiMap<L, R> {
    left_to_right: HashMap<L, R>,
    right_to_left: HashMap<R, L>,
}

impl<L: Eq + std::hash::Hash + Clone, R: Eq + std::hash::Hash + Clone> BiMap<L, R> {
    pub fn new() -> Self {
        BiMap {
            left_to_right: HashMap::new(),
            right_to_left: HashMap::new(),
        }
    }

    pub fn from<const N: usize>(pairs: [(L, R); N]) -> Self {
        let mut this = BiMap::new();
        for (left, right) in pairs {
            this.insert(left, right);
        }
        this
    }

    pub fn len(&self) -> usize {
        self.left_to_right.len()
    }

    pub fn into_inner(self) -> (HashMap<L, R>, HashMap<R, L>) {
        (self.left_to_right, self.right_to_left)
    }

    pub fn into_reversed(self) -> BiMap<R, L> {
        BiMap {
            left_to_right: self.right_to_left,
            right_to_left: self.left_to_right,
        }
    }

    pub fn into_left_map(self) -> HashMap<L, R> {
        self.left_to_right
    }

    pub fn into_right_map(self) -> HashMap<R, L> {
        self.right_to_left
    }

    pub fn insert(&mut self, left: L, right: R) {
        // Ensure no existing mapping for left or right
        if self.left_to_right.contains_key(&left) || self.right_to_left.contains_key(&right) {
            panic!("Cannot insert: left or right already exists in the map");
        }
        self.left_to_right.insert(left.clone(), right.clone());
        self.right_to_left.insert(right, left);
    }

    pub fn get_left(&self, left: &L) -> Option<&R> {
        self.left_to_right.get(left)
    }

    pub fn get_right(&self, right: &R) -> Option<&L> {
        self.right_to_left.get(right)
    }

    pub fn contains_left(&self, left: &L) -> bool {
        self.left_to_right.contains_key(left)
    }

    pub fn contains_right(&self, right: &R) -> bool {
        self.right_to_left.contains_key(right)
    }

    pub fn remove_left(&mut self, left: &L) -> Option<R> {
        if let Some(right) = self.left_to_right.remove(left) {
            self.right_to_left.remove(&right);
            Some(right)
        } else {
            None
        }
    }

    pub fn remove_right(&mut self, right: &R) -> Option<L> {
        if let Some(left) = self.right_to_left.remove(right) {
            self.left_to_right.remove(&left);
            Some(left)
        } else {
            None
        }
    }

    pub fn right_values(&self) -> impl Iterator<Item = &R> {
        self.left_to_right.values()
    }

    pub fn left_values(&self) -> impl Iterator<Item = &L> {
        self.right_to_left.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&L, &R)> {
        self.left_to_right.iter()
    }
}

// implement IntoIterator for BiMap
impl<L: Eq + std::hash::Hash + Clone, R: Eq + std::hash::Hash + Clone> IntoIterator
    for BiMap<L, R>
{
    type Item = (L, R);
    type IntoIter = std::collections::hash_map::IntoIter<L, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.left_to_right.into_iter()
    }
}
