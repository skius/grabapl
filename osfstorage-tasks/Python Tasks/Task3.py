# Binary Search Tree Serialisation
# The function f takes as input the root of a binary search tree.
# It should return an ordered list of the elements of the tree.
# Reminder: A binary search tree is a tree where for each node, all the values in the left subtree are smaller, and
# all the values in the right subtree are greater than the node's number value.

class Node:
    def __init__(self, value, left=None, right=None):
        self.value = value
        self.left = left
        self.right = right
    def __str__(self):
        if not self.left and not self.right:
            return f'{self.value}'
        return f'({self.value}: {self.left or ""},{self.right or ""})'
    def __eq__(self, other):
        if other is None: return False
        if self.value != other.value: return False
        cs = [c for c in [self.left, self.right] if c]
        co = [c for c in [other.left, other.right] if c]
        return len(cs) == len(co) and all([a == b for (a, b) in zip(cs, co)])

def f(node):
    if not node: return []
    return f(node.left) + f(node.right) + [node.value]
